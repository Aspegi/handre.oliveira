//! Regression tests: prevencao estrutural dos 9 bugs historicos.
//!
//! Cada teste mapeia para um bug que ja aconteceu e custou uma sessao.
//! Se algum teste voltar a falhar, o bug regressou.
//!
//! Nota: estes testes leem o codigo-fonte e procuram patterns proibidos.
//! Sao "contract tests" - garantem que as correcoes historicas se mantem.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .to_path_buf()
}

fn read(path: &str) -> String {
    let p = repo_root().join(path);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("leitura {}: {}", p.display(), e))
}

// ============ Bug #1: deadband_radius (v0.4 - drone a 15.24m) ============
#[test]
fn bug_01_no_deadband_radius() {
    let src = read("crates/rnfc-core/src/guidance.rs").to_lowercase();
    assert!(!src.contains("deadband"),
        "BUG #1 REGRESSOU: deadband_radius detectado em guidance.rs. \
         Colocou o drone a 15.24m do alvo em v0.4.0. \
         Se precisas de yaw=0 perto do alvo, usa hover/return-to-launch.");
}

// ============ Bug #2: direct_mode { 8.0 } (v0.4 - arrival a 8m) ============
#[test]
fn bug_02_no_direct_mode_8m() {
    let src = read("crates/rnfc-backend-sitl/src/main.rs");
    assert!(!src.contains("if direct_mode { 8.0 }"),
        "BUG #2 REGRESSOU: 'if direct_mode {{ 8.0 }}' em main.rs. \
         Isto fazia arrival disparar a 8m em vez de arrival_radius. \
         arrival_radius deve ser uniforme em todos os waypoints.");
}

// ============ Bug #3: min speed 3.0 perto do goal (v0.4 - overshoot 25m) ============
#[test]
fn bug_03_no_min_speed_3_perto_goal() {
    let src = read("crates/rnfc-backend-sitl/src/main.rs");
    assert!(!src.contains("max(3.0)"),
        "BUG #3 REGRESSOU: '.max(3.0)' em main.rs. \
         Minimo de 3 m/s perto do goal causava overshoot (25m no v0.4). \
         Usa .max(0.5) ou menor.");
}

// ============ Bug #4: powi em no_std (v1.3 - E0599) ============
#[test]
fn bug_04_no_powi_in_fdir() {
    let src = read("crates/rnfc-fdir/src/lib.rs");
    assert!(!src.contains(".powi("),
        "BUG #4 REGRESSOU: '.powi()' em rnfc-fdir. \
         powi nao existe em no_std. Usa x*x ou libm::powf.");
}

// ============ Bug #5: sqrt sem libm (v1.3 - E0599) ============
#[test]
fn bug_05_no_raw_sqrt_in_fdir() {
    let src = read("crates/rnfc-fdir/src/lib.rs");
    assert!(!src.contains(".sqrt()"),
        "BUG #5 REGRESSOU: '.sqrt()' em rnfc-fdir. \
         f32::sqrt nao existe em no_std. Usa libm::sqrtf().");
}

// ============ Bug #6: plan_astar loop sem guard (v0.9.7 - REVERTIDO) ============
#[test]
fn bug_06_no_unbounded_reroute_loop() {
    let src = read("crates/rnfc-backend-sitl/src/main.rs");
    assert!(!src.contains("(stuck, a replanear"),
        "BUG #6 REGRESSOU: loop de replan sem guard em main.rs. \
         Causava 7 replans em 30s. Foi revertido em e15d48f.");
}

// ============ Bug #7: use libm antes dos inner attrs (v1.3 - E0753) ============
#[test]
fn bug_07_use_apos_inner_attrs() {
    let src = read("crates/rnfc-fdir/src/lib.rs");
    let lines: Vec<&str> = src.lines().collect();
    let mut last_attr = 0;
    let mut first_use = lines.len();
    let mut first_doc = lines.len();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("#![") { last_attr = i; }
        if line.starts_with("//!") && first_doc == lines.len() { first_doc = i; }
        if line.starts_with("use ") && first_use == lines.len() { first_use = i; }
    }
    assert!(first_use > last_attr,
        "BUG #7 REGRESSOU: 'use' antes de inner attrs (E0753). \
         Ordem obrigatoria: #![...] -> //! -> use -> codigo.");
    if first_doc < lines.len() {
        assert!(first_use > first_doc,
            "BUG #7 REGRESSOU: 'use' antes de //! docs.");
    }
}

// ============ Bug #8: derives duplicados (v1.4 - E0119) ============
#[test]
fn bug_08_no_duplicate_derives() {
    let src = read("crates/rnfc-abstractions/src/lib.rs");
    let lines: Vec<&str> = src.lines().collect();
    let mut prev_derive = false;
    for (i, line) in lines.iter().enumerate() {
        let is_derive = line.trim_start().starts_with("#[derive(");
        assert!(!(is_derive && prev_derive),
            "BUG #8 REGRESSOU: derives duplicados na linha {}. \
             Cada struct deve ter no maximo 1 #[derive(...)].",
            i + 1);
        prev_derive = is_derive;
    }
}

// ============ Bug #9: clippy::map_or invalido (v1.8 - unknown_lint) ============
#[test]
fn bug_09_no_invalid_clippy_lint() {
    let src = read("crates/rnfc-backend-sitl/src/main.rs");
    assert!(!src.contains("clippy::map_or)"),
        "BUG #9 REGRESSOU: 'clippy::map_or' (sem 'unnecessary' ou 'manual'). \
         Lint invalido. Usa clippy::unnecessary_map_or ou clippy::manual_map.");
}
