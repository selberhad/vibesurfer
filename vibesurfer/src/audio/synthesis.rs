//! Procedural music synthesis configuration.

/// Glicol composition (procedural music code)
pub const GLICOL_COMPOSITION: &str = r#"
~gate: speed 2.0 >> seq 60 _60 _~a 48
~a: choose 48 48 48 72 0 0 0
~amp: ~gate >> envperc 0.001 0.1
~pit: ~gate >> mul 261.63
~lead: saw ~pit >> mul ~amp >> lpf ~mod 5.0 >> mul 0.1
~mod: sin 0.2 >> mul 1300 >> add 1500
o: ~lead >> plate 0.1
"#;
