import Lake
open Lake DSL

package «solisp-verify» where
  version := v!"0.1.0"
  description := "Formal verification library for Solisp programs"

lean_lib «Solisp» where
  roots := #[`Solisp]

@[default_target]
lean_exe «solisp-verify» where
  root := `Main
  supportInterpreter := true
