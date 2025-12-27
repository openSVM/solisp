import Lake
open Lake DSL

package «ovsm-verify» where
  version := v!"0.1.0"
  description := "Formal verification library for OVSM programs"

lean_lib «OVSM» where
  roots := #[`OVSM]

@[default_target]
lean_exe «ovsm-verify» where
  root := `Main
  supportInterpreter := true
