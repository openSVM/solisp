/-
  AEA Protocol Formal Specification
  =================================
  
  This file provides a formal specification of the AEA (Autonomous Economic Agents)
  protocol in Lean 4. It defines:
  
  1. Types and state structures
  2. State machine transitions
  3. Invariants that must always hold
  4. Access control predicates
  5. Economic properties
  
  This specification can be used to:
  - Verify the implementation matches the spec
  - Prove invariants are preserved by all transitions
  - Detect missing checks or invalid state transitions
-/

import OVSM.Primitives
import OVSM.Solana

namespace AEA

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 1: BASIC TYPES
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Participant type enumeration -/
inductive ParticipantType where
  | User      : ParticipantType  -- 0: Human, no stake required
  | Agent     : ParticipantType  -- 1: AI agent, must stake
  | Provider  : ParticipantType  -- 2: Service provider, must stake
  | Validator : ParticipantType  -- 3: Dispute validator, high stake
  deriving DecidableEq, Repr

/-- Participant status enumeration -/
inductive ParticipantStatus where
  | Inactive  : ParticipantStatus  -- 0: Not registered or deactivated
  | Active    : ParticipantStatus  -- 1: Normal operating state
  | Cooldown  : ParticipantStatus  -- 2: Waiting to unstake
  | Slashed   : ParticipantStatus  -- 3: Penalized for violation
  | Suspended : ParticipantStatus  -- 4: Temporarily suspended by admin
  deriving DecidableEq, Repr

/-- Order status enumeration - this is the critical state machine -/
inductive OrderStatus where
  | Created    : OrderStatus  -- 0: Order created, funds escrowed
  | Accepted   : OrderStatus  -- 1: Provider accepted
  | InProgress : OrderStatus  -- 2: Work in progress
  | Delivered  : OrderStatus  -- 3: Provider submitted delivery
  | Completed  : OrderStatus  -- 4: Buyer confirmed, payment released
  | Disputed   : OrderStatus  -- 5: Dispute opened
  | Refunded   : OrderStatus  -- 6: Funds returned to buyer
  | Cancelled  : OrderStatus  -- 7: Order cancelled before acceptance
  deriving DecidableEq, Repr

/-- Public key type (32 bytes) -/
structure Pubkey where
  bytes : ByteArray
  size_eq : bytes.size = 32
  deriving Repr

/-- Amount type (non-negative) -/
def Amount := { n : UInt64 // true }  -- Could add non-negative constraint

/-- Timestamp type -/
def Timestamp := UInt64

/-- Basis points (0-10000 for 0%-100%) -/
def BasisPoints := { n : UInt64 // n ≤ 10000 }

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 2: ACCOUNT STRUCTURES
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Protocol configuration account -/
structure ProtocolConfig where
  initialized       : Bool
  min_agent_stake   : UInt64
  min_provider_stake: UInt64
  min_validator_stake: UInt64
  escrow_fee_bps    : UInt64      -- Basis points (e.g., 250 = 2.5%)
  cooldown_seconds  : UInt64
  dispute_window    : UInt64
  total_participants: UInt64
  total_staked      : UInt64
  total_volume      : UInt64
  admin             : Pubkey
  deriving Repr

/-- Participant account -/
structure Participant where
  participant_type  : ParticipantType
  status            : ParticipantStatus
  stake_amount      : UInt64
  reputation_score  : Int64       -- Can be negative
  tasks_completed   : UInt64
  tasks_failed      : UInt64
  disputes_won      : UInt64
  disputes_lost     : UInt64
  total_earned      : UInt64
  total_spent       : UInt64
  registered_at     : Timestamp
  last_active       : Timestamp
  cooldown_start    : Timestamp
  authority         : Pubkey
  endpoint          : ByteArray   -- 64 bytes
  display_name      : ByteArray   -- 32 bytes
  capabilities_hash : ByteArray   -- 32 bytes
  deriving Repr

/-- Service listing -/
structure Service where
  is_active         : Bool
  service_id        : UInt64
  price             : UInt64
  min_reputation    : Int64
  max_concurrent    : UInt64
  active_orders     : UInt64
  completed_orders  : UInt64
  created_at        : Timestamp
  provider          : Pubkey
  description_hash  : ByteArray   -- 64 bytes
  category_hash     : ByteArray   -- 32 bytes
  deriving Repr

/-- Order/Escrow account -/
structure Order where
  status            : OrderStatus
  order_id          : UInt64
  service_id        : UInt64
  amount            : UInt64      -- Escrowed payment
  fee_amount        : UInt64      -- Protocol fee
  created_at        : Timestamp
  accepted_at       : Timestamp
  delivered_at      : Timestamp
  deadline          : Timestamp
  dispute_deadline  : Timestamp
  buyer             : Pubkey
  provider          : Pubkey
  request_hash      : ByteArray   -- 64 bytes
  delivery_hash     : ByteArray   -- 64 bytes
  deriving Repr

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 3: GLOBAL STATE
-- ═══════════════════════════════════════════════════════════════════════════════

/-- The complete protocol state -/
structure ProtocolState where
  config       : ProtocolConfig
  participants : List Participant
  services     : List Service
  orders       : List Order
  -- Token balances (simplified model)
  vault_balance: UInt64           -- Protocol vault
  escrow_total : UInt64           -- Total in escrow accounts
  deriving Repr

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 4: ORDER STATE MACHINE
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Valid order status transitions -/
def validOrderTransition : OrderStatus → OrderStatus → Bool
  -- From Created
  | .Created, .Accepted   => true   -- Provider accepts
  | .Created, .Cancelled  => true   -- Buyer cancels before acceptance
  -- From Accepted
  | .Accepted, .InProgress => true  -- Work begins
  | .Accepted, .Refunded   => true  -- Provider refunds
  -- From InProgress
  | .InProgress, .Delivered => true -- Provider submits
  | .InProgress, .Disputed  => true -- Either party disputes
  | .InProgress, .Refunded  => true -- Provider refunds
  -- From Delivered
  | .Delivered, .Completed => true  -- Buyer confirms
  | .Delivered, .Disputed  => true  -- Buyer disputes
  -- From Disputed
  | .Disputed, .Completed => true   -- Resolved in provider's favor
  | .Disputed, .Refunded  => true   -- Resolved in buyer's favor
  -- No other transitions allowed
  | _, _ => false

/-- Terminal states (no further transitions) -/
def isTerminalOrderStatus : OrderStatus → Bool
  | .Completed => true
  | .Refunded  => true
  | .Cancelled => true
  | _ => false

/-- Theorem: Terminal states have no valid outgoing transitions -/
theorem terminal_states_are_final (s : OrderStatus) :
    isTerminalOrderStatus s = true → 
    ∀ s', validOrderTransition s s' = false := by
  intro h
  intro s'
  cases s <;> simp [isTerminalOrderStatus] at h <;> 
  cases s' <;> simp [validOrderTransition]

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 5: PARTICIPANT STATE MACHINE
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Valid participant status transitions -/
def validParticipantTransition : ParticipantStatus → ParticipantStatus → Bool
  -- From Inactive (registration)
  | .Inactive, .Active    => true   -- Successful registration
  -- From Active
  | .Active, .Cooldown    => true   -- Start unstaking
  | .Active, .Slashed     => true   -- Violation penalty
  | .Active, .Suspended   => true   -- Admin suspension
  -- From Cooldown
  | .Cooldown, .Inactive  => true   -- Complete unstake
  | .Cooldown, .Active    => true   -- Cancel unstake
  | .Cooldown, .Slashed   => true   -- Violation during cooldown
  -- From Suspended
  | .Suspended, .Active   => true   -- Suspension lifted
  | .Suspended, .Slashed  => true   -- Slashed while suspended
  -- Slashed is terminal
  | .Slashed, _           => false
  -- No other transitions
  | _, _ => false

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 6: ECONOMIC INVARIANTS
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Sum of all participant stakes -/
def totalParticipantStakes (ps : List Participant) : UInt64 :=
  ps.foldl (fun acc p => acc + p.stake_amount) 0

/-- Sum of all active order escrows -/
def totalActiveEscrow (os : List Order) : UInt64 :=
  os.foldl (fun acc o => 
    if ¬isTerminalOrderStatus o.status 
    then acc + o.amount + o.fee_amount 
    else acc) 0

/-- INVARIANT 1: Stake accounting
    The config's total_staked must equal sum of all participant stakes -/
def stakeAccountingInvariant (s : ProtocolState) : Prop :=
  s.config.total_staked = totalParticipantStakes s.participants

/-- INVARIANT 2: Escrow accounting  
    Total escrow balance must equal sum of non-terminal order amounts -/
def escrowAccountingInvariant (s : ProtocolState) : Prop :=
  s.escrow_total = totalActiveEscrow s.orders

/-- INVARIANT 3: Participant count
    Config participant count matches actual participants -/
def participantCountInvariant (s : ProtocolState) : Prop :=
  s.config.total_participants = s.participants.length

/-- INVARIANT 4: No negative stakes -/
def nonNegativeStakesInvariant (s : ProtocolState) : Prop :=
  ∀ p ∈ s.participants, p.stake_amount ≥ 0

/-- INVARIANT 5: Active orders within service limits -/
def serviceOrderLimitInvariant (s : ProtocolState) : Prop :=
  ∀ svc ∈ s.services, svc.active_orders ≤ svc.max_concurrent

/-- INVARIANT 6: Stake requirements by type -/
def stakeRequirementInvariant (s : ProtocolState) : Prop :=
  ∀ p ∈ s.participants,
    match p.participant_type, p.status with
    | .User, _ => true  -- Users don't need stake
    | .Agent, .Active => p.stake_amount ≥ s.config.min_agent_stake
    | .Provider, .Active => p.stake_amount ≥ s.config.min_provider_stake
    | .Validator, .Active => p.stake_amount ≥ s.config.min_validator_stake
    | _, _ => true  -- Non-active don't need to meet requirement

/-- INVARIANT 7: Fee calculation correctness -/
def feeCalculationInvariant (s : ProtocolState) : Prop :=
  ∀ o ∈ s.orders,
    o.fee_amount = (o.amount * s.config.escrow_fee_bps) / 10000

/-- INVARIANT 8: Order timestamps consistency -/
def orderTimestampInvariant (s : ProtocolState) : Prop :=
  ∀ o ∈ s.orders,
    o.created_at ≤ o.accepted_at ∧
    o.accepted_at ≤ o.delivered_at ∧
    o.created_at ≤ o.deadline

/-- All invariants combined -/
def allInvariants (s : ProtocolState) : Prop :=
  stakeAccountingInvariant s ∧
  escrowAccountingInvariant s ∧
  participantCountInvariant s ∧
  nonNegativeStakesInvariant s ∧
  serviceOrderLimitInvariant s ∧
  stakeRequirementInvariant s ∧
  feeCalculationInvariant s ∧
  orderTimestampInvariant s

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 7: ACCESS CONTROL PREDICATES
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Check if pubkey matches participant authority -/
def isParticipantAuthority (p : Participant) (signer : Pubkey) : Bool :=
  p.authority == signer

/-- Check if pubkey is order buyer -/
def isOrderBuyer (o : Order) (signer : Pubkey) : Bool :=
  o.buyer == signer

/-- Check if pubkey is order provider -/
def isOrderProvider (o : Order) (signer : Pubkey) : Bool :=
  o.provider == signer

/-- Check if pubkey is admin -/
def isAdmin (s : ProtocolState) (signer : Pubkey) : Bool :=
  s.config.admin == signer

/-- ACCESS CONTROL: InitializeProtocol
    - Can only be called once (not initialized)
    - Caller becomes admin -/
def canInitialize (s : ProtocolState) : Prop :=
  ¬s.config.initialized

/-- ACCESS CONTROL: RegisterUser/Agent/Provider/Validator
    - Protocol must be initialized
    - Participant must not already exist
    - Signer must be the authority -/
def canRegister (s : ProtocolState) (authority : Pubkey) : Prop :=
  s.config.initialized ∧
  ¬∃ p ∈ s.participants, p.authority == authority

/-- ACCESS CONTROL: UpdateProfile
    - Participant must exist and be active
    - Signer must be the authority -/
def canUpdateProfile (s : ProtocolState) (p : Participant) (signer : Pubkey) : Prop :=
  p.status = .Active ∧
  isParticipantAuthority p signer

/-- ACCESS CONTROL: CreateOrder
    - Buyer must be active
    - Provider must be active
    - Service must be active
    - Buyer reputation must meet minimum -/
def canCreateOrder (s : ProtocolState) (buyer : Participant) 
    (provider : Participant) (svc : Service) : Prop :=
  buyer.status = .Active ∧
  provider.status = .Active ∧
  svc.is_active ∧
  buyer.reputation_score ≥ svc.min_reputation ∧
  svc.active_orders < svc.max_concurrent

/-- ACCESS CONTROL: AcceptOrder
    - Order must be in Created state
    - Signer must be the provider -/
def canAcceptOrder (o : Order) (signer : Pubkey) : Prop :=
  o.status = .Created ∧
  isOrderProvider o signer

/-- ACCESS CONTROL: ConfirmDelivery
    - Order must be in Delivered state
    - Signer must be the buyer -/
def canConfirmDelivery (o : Order) (signer : Pubkey) : Prop :=
  o.status = .Delivered ∧
  isOrderBuyer o signer

/-- ACCESS CONTROL: CancelOrder
    - Order must be in Created state (not yet accepted)
    - Signer must be the buyer -/
def canCancelOrder (o : Order) (signer : Pubkey) : Prop :=
  o.status = .Created ∧
  isOrderBuyer o signer

/-- ACCESS CONTROL: OpenDispute
    - Order must be in disputable state
    - Signer must be buyer or provider
    - Must be within dispute window -/
def canOpenDispute (o : Order) (signer : Pubkey) (now : Timestamp) : Prop :=
  (o.status = .Delivered ∨ o.status = .InProgress) ∧
  (isOrderBuyer o signer ∨ isOrderProvider o signer) ∧
  now ≤ o.dispute_deadline

/-- ACCESS CONTROL: ResolveDispute
    - Order must be in Disputed state
    - Signer must be admin or validator -/
def canResolveDispute (s : ProtocolState) (o : Order) (signer : Pubkey) : Prop :=
  o.status = .Disputed ∧
  isAdmin s signer

/-- ACCESS CONTROL: SlashParticipant
    - Signer must be admin -/
def canSlash (s : ProtocolState) (signer : Pubkey) : Prop :=
  isAdmin s signer

/-- ACCESS CONTROL: SuspendParticipant
    - Signer must be admin
    - Participant must not already be slashed -/
def canSuspend (s : ProtocolState) (p : Participant) (signer : Pubkey) : Prop :=
  isAdmin s signer ∧
  p.status ≠ .Slashed

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 8: STATE TRANSITION FUNCTIONS
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Effect of CreateOrder on state -/
def createOrderEffect (s : ProtocolState) (buyer : Participant) 
    (provider : Participant) (svc : Service) (amount : UInt64) 
    (now : Timestamp) : ProtocolState :=
  let fee := (amount * s.config.escrow_fee_bps) / 10000
  let total := amount + fee
  let newOrder : Order := {
    status := .Created
    order_id := now  -- Simplified: use timestamp as ID
    service_id := svc.service_id
    amount := amount
    fee_amount := fee
    created_at := now
    accepted_at := 0
    delivered_at := 0
    deadline := 0
    dispute_deadline := now + s.config.dispute_window
    buyer := buyer.authority
    provider := provider.authority
    request_hash := ByteArray.empty
    delivery_hash := ByteArray.empty
  }
  { s with
    orders := newOrder :: s.orders
    escrow_total := s.escrow_total + total
  }

/-- Effect of ConfirmDelivery on state -/
def confirmDeliveryEffect (s : ProtocolState) (o : Order) : ProtocolState :=
  let updatedOrder := { o with status := .Completed }
  let orders' := s.orders.map (fun x => if x.order_id = o.order_id then updatedOrder else x)
  { s with
    orders := orders'
    escrow_total := s.escrow_total - o.amount - o.fee_amount
    config := { s.config with total_volume := s.config.total_volume + o.amount }
  }

/-- Effect of CancelOrder on state (before acceptance) -/
def cancelOrderEffect (s : ProtocolState) (o : Order) : ProtocolState :=
  let updatedOrder := { o with status := .Cancelled }
  let orders' := s.orders.map (fun x => if x.order_id = o.order_id then updatedOrder else x)
  { s with
    orders := orders'
    escrow_total := s.escrow_total - o.amount - o.fee_amount
  }

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 9: CORRECTNESS THEOREMS
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Theorem: CreateOrder preserves escrow accounting -/
theorem createOrder_preserves_escrow (s : ProtocolState) 
    (buyer provider : Participant) (svc : Service) (amount : UInt64) (now : Timestamp) :
    escrowAccountingInvariant s →
    escrowAccountingInvariant (createOrderEffect s buyer provider svc amount now) := by
  sorry  -- Proof would verify escrow_total is updated correctly

/-- Theorem: ConfirmDelivery reduces escrow correctly -/
theorem confirmDelivery_preserves_escrow (s : ProtocolState) (o : Order) :
    escrowAccountingInvariant s →
    o ∈ s.orders →
    o.status = .Delivered →
    escrowAccountingInvariant (confirmDeliveryEffect s o) := by
  sorry

/-- Theorem: Order state machine is respected -/
theorem order_transitions_valid (s s' : ProtocolState) (o o' : Order) :
    o ∈ s.orders →
    o' ∈ s'.orders →
    o.order_id = o'.order_id →
    o.status ≠ o'.status →
    validOrderTransition o.status o'.status = true := by
  sorry

/-- Theorem: Terminal orders never change -/
theorem terminal_orders_immutable (s s' : ProtocolState) (o o' : Order) :
    o ∈ s.orders →
    o' ∈ s'.orders →
    o.order_id = o'.order_id →
    isTerminalOrderStatus o.status = true →
    o.status = o'.status := by
  sorry

/-- Theorem: Only authorized parties can transition orders -/
theorem order_access_control (s : ProtocolState) (o : Order) (signer : Pubkey) :
    -- AcceptOrder requires provider signature
    (o.status = .Created → o.status = .Accepted → isOrderProvider o signer) ∧
    -- ConfirmDelivery requires buyer signature
    (o.status = .Delivered → o.status = .Completed → isOrderBuyer o signer) ∧
    -- CancelOrder requires buyer signature
    (o.status = .Created → o.status = .Cancelled → isOrderBuyer o signer) := by
  sorry

/-- Theorem: Slashing reduces stake -/
theorem slashing_reduces_stake (s s' : ProtocolState) (p p' : Participant) 
    (slash_pct : UInt64) :
    p ∈ s.participants →
    p' ∈ s'.participants →
    p.authority == p'.authority →
    slash_pct ≤ 10000 →
    -- After slashing, stake is reduced
    p'.stake_amount ≤ p.stake_amount := by
  sorry

/-- Theorem: Balance conservation 
    Total tokens in system = vault + escrow + user balances (constant) -/
theorem balance_conservation (s s' : ProtocolState) :
    -- The sum of vault + escrow + external balances is constant
    -- This requires modeling external token accounts
    s.vault_balance + s.escrow_total = s'.vault_balance + s'.escrow_total := by
  sorry

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 10: SAFETY PROPERTIES
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Safety: No double-spend of escrowed funds -/
def noDoubleSpend (s : ProtocolState) : Prop :=
  ∀ o ∈ s.orders,
    isTerminalOrderStatus o.status = true →
    -- Once terminal, funds have been disbursed exactly once
    True  -- Would need more detailed accounting

/-- Safety: Reputation can't overflow -/
def reputationBounded (s : ProtocolState) : Prop :=
  ∀ p ∈ s.participants,
    p.reputation_score ≥ -1000000 ∧ p.reputation_score ≤ 1000000

/-- Safety: Cooldown must complete before unstaking -/
def cooldownRespected (s : ProtocolState) (now : Timestamp) : Prop :=
  ∀ p ∈ s.participants,
    p.status = .Cooldown →
    -- Can only transition to Inactive if cooldown elapsed
    True  -- Would check: now ≥ p.cooldown_start + s.config.cooldown_seconds

/-- Safety: Disputes must be resolved within window -/  
def disputeWindowRespected (s : ProtocolState) (now : Timestamp) : Prop :=
  ∀ o ∈ s.orders,
    o.status = .Disputed →
    now ≤ o.dispute_deadline + s.config.dispute_window

-- ═══════════════════════════════════════════════════════════════════════════════
-- SECTION 11: LIVENESS PROPERTIES
-- ═══════════════════════════════════════════════════════════════════════════════

/-- Liveness: Every order eventually terminates
    (Assuming good faith participation or timeout mechanisms) -/
def orderEventuallyTerminates (o : Order) (timeout : Timestamp) : Prop :=
  -- If deadline passes and no action, order auto-refunds
  True  -- Would need temporal logic

/-- Liveness: Unstaking eventually succeeds
    (If cooldown completes without slashing) -/
def unstakingEventuallySucceeds (p : Participant) (s : ProtocolState) : Prop :=
  p.status = .Cooldown →
  -- After cooldown_seconds, can complete unstake
  True

end AEA
