//! SCC demo: compile safety requirements into 3 artifacts.
//! Run: cargo run -p rnfc-scc --example scc_demo

use rnfc_scc::{compile_requirements, generate_gsn, generate_test, parse_requirements};
use std::fs;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SCC Demo: Safety Case Compiler ===\n");

    // 1. Sample requirements (would be a file in production)
    let input = "# UAV safety requirements\nspeed_max: 10.0 HIGH\naltitude_range: 5.0 120.0 HIGH\nobstacle_clearance: 2.0 MEDIUM\n";
    fs::write("requirements.safety", input)?;
    println!("[1] Wrote requirements.safety ({} bytes)", input.len());

    // 2. Parse
    let reqs = parse_requirements(input)?;
    println!("[2] Parsed {} requirements", reqs.len());

    // 3. Generate test stubs
    let mut tests = String::new();
    for r in &reqs {
        tests.push_str(&generate_test(r));
        tests.push('\n');
    }
    fs::write("generated_tests.rs", &tests)?;
    println!("[3] Generated generated_tests.rs ({} bytes)", tests.len());

    // 4. Compile chain
    let chain = compile_requirements(&reqs)?;
    println!(
        "[4] Compiled chain: {} blocks, root=0x{:016x}",
        chain.block_count(),
        chain.root_hash()
    );
    assert!(chain.verify(), "chain failed verification");

    // 5. Generate GSN safety case
    let mut gsn = String::from("# Safety Case (GSN)\n\n");
    for (i, r) in reqs.iter().enumerate() {
        gsn.push_str(&generate_gsn(r, i));
    }
    fs::write("safety_case_gsn.md", &gsn)?;
    println!("[5] Generated safety_case_gsn.md ({} bytes)", gsn.len());

    // 6. Regulator verification
    let verified = chain.verify();
    println!("\n=== Regulator Verdict ===");
    println!("Chain verified: {}", if verified { "PASS" } else { "FAIL" });
    println!("Artifacts: requirements.safety, generated_tests.rs, safety_case_gsn.md");

    let mut f = fs::File::create("scc_demo_output.txt")?;
    writeln!(
        f,
        "SCC compiled {} requirements into chain root 0x{:016x}",
        reqs.len(),
        chain.root_hash()
    )?;

    Ok(())
}
