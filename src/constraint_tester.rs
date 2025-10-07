use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::{Field, One};
use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_poly::univariate::DensePolynomial;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::rand::SeedableRng;
use blake2::Blake2s;
use rand_chacha::ChaCha20Rng;
use ark_marlin::{Marlin, SimpleHashFiatShamirRng};
use ark_relations::lc;
use std::env;
use std::time::Instant;
use serde_json;

// Type aliases for easier use
type MarlinPC = MarlinKZG10<Bls12_381, DensePolynomial<Fr>>;
type MarlinFS = SimpleHashFiatShamirRng<Blake2s, ChaCha20Rng>;
type MarlinInst = Marlin<Fr, MarlinPC, MarlinFS>;

// 可配置的约束电路
#[derive(Clone, Debug)]
struct ConfigurableCircuit {
    // 私有输入（见证）
    witness_a: Option<Fr>,
    witness_b: Option<Fr>,
    // 公开输入
    public_c: Option<Fr>,
    // 约束类型：期望的结果
    expected_result: Fr,
}

impl ConstraintSynthesizer<Fr> for ConfigurableCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        // 分配私有变量
        let a = cs.new_witness_variable(|| self.witness_a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.witness_b.ok_or(SynthesisError::AssignmentMissing))?;
        
        // 分配公开变量
        let c = cs.new_input_variable(|| self.public_c.ok_or(SynthesisError::AssignmentMissing))?;

        // 关键！这里强制约束必须是 a × b = expected_result
        // 而不是 a × b = c，这样我们可以测试不同的约束
        let expected_var = cs.new_witness_variable(|| Ok(self.expected_result))?;
        
        // 真实的约束：a × b = expected_result
        cs.enforce_constraint(
            lc!() + a,
            lc!() + b, 
            lc!() + expected_var
        )?;
        
        // 额外约束：如果用户声称的c不等于expected_result，这会导致证明失败
        // 但我们需要让约束系统知道这个关系
        // 这里我们强制 c 必须等于 expected_result
        let one_var = cs.new_witness_variable(|| Ok(Fr::one()))?;
        cs.enforce_constraint(
            lc!() + c,
            lc!() + one_var,
            lc!() + expected_var
        )?;

        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <operation> [args...]", args[0]);
        eprintln!("Operations:");
        eprintln!("  test_constraint <witness_a> <witness_b> <claimed_c>");
        eprintln!("  verify_constraint <witness_a> <witness_b> <claimed_c>");
        return;
    }

    let mut rng = ChaCha20Rng::seed_from_u64(0u64);
    
    match args[1].as_str() {
        "test_constraint" => {
            if args.len() != 5 {
                eprintln!("Usage: {} test_constraint <witness_a> <witness_b> <claimed_c>", args[0]);
                return;
            }
            
            let witness_a: u64 = args[2].parse().unwrap();
            let witness_b: u64 = args[3].parse().unwrap();
            let claimed_c: u64 = args[4].parse().unwrap();
            
            println!("🧪 测试约束: {} × {} =? {}", witness_a, witness_b, claimed_c);
            
            let actual_result = witness_a * witness_b;
            let is_constraint_correct = actual_result == claimed_c;
            
            println!("🔍 实际计算: {} × {} = {}", witness_a, witness_b, actual_result);
            println!("🎯 用户声称: {}", claimed_c);
            println!("📊 约束正确性: {}", if is_constraint_correct { "✅ 正确" } else { "❌ 错误" });
            
            // 构建电路 - 关键：使用actual_result作为期望值
            let circuit = ConfigurableCircuit {
                witness_a: Some(Fr::from(witness_a)),
                witness_b: Some(Fr::from(witness_b)),
                public_c: Some(Fr::from(claimed_c)),
                expected_result: Fr::from(actual_result), // 这里是关键！
            };
            
            let start_time = Instant::now();
            
            // 执行完整的证明流程
            match run_full_proof_system(circuit, &mut rng) {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    
                    let final_result = serde_json::json!({
                        "test_complete": true,
                        "constraint": format!("{} × {} = {}", witness_a, witness_b, claimed_c),
                        "actual_result": actual_result,
                        "claimed_result": claimed_c,
                        "is_constraint_mathematically_correct": is_constraint_correct,
                        "zksnark_proof_valid": result.proof_valid,
                        "verification_passed": result.verification_passed,
                        "explanation": if is_constraint_correct {
                            "约束数学上正确，zkSNARK证明应该通过"
                        } else {
                            "约束数学上错误，zkSNARK证明应该失败"
                        },
                        "timing": {
                            "total_ms": duration.as_millis(),
                            "setup_ms": result.setup_time,
                            "index_ms": result.index_time,
                            "prove_ms": result.prove_time,
                            "verify_ms": result.verify_time
                        },
                        "rust_execution": true
                    });
                    
                    println!("📊 最终结果:");
                    println!("{}", serde_json::to_string_pretty(&final_result).unwrap());
                },
                Err(e) => {
                    let error_result = serde_json::json!({
                        "test_complete": false,
                        "error": format!("{:?}", e),
                        "constraint": format!("{} × {} = {}", witness_a, witness_b, claimed_c)
                    });
                    println!("{}", serde_json::to_string_pretty(&error_result).unwrap());
                }
            }
        },
        
        "verify_constraint" => {
            if args.len() != 5 {
                eprintln!("Usage: {} verify_constraint <witness_a> <witness_b> <claimed_c>", args[0]);
                return;
            }
            
            let witness_a: u64 = args[2].parse().unwrap();
            let witness_b: u64 = args[3].parse().unwrap();
            let claimed_c: u64 = args[4].parse().unwrap();
            
            println!("🔍 验证约束: {} × {} =? {}", witness_a, witness_b, claimed_c);
            
            let actual_result = witness_a * witness_b;
            let is_valid = actual_result == claimed_c;
            
            let result = serde_json::json!({
                "verification_complete": true,
                "constraint": format!("{} × {} = {}", witness_a, witness_b, claimed_c),
                "is_valid": is_valid,
                "actual_result": actual_result,
                "claimed_result": claimed_c,
                "message": if is_valid {
                    "约束验证通过 - 数学关系正确".to_string()
                } else {
                    format!("约束验证失败 - {} × {} = {}, 不等于 {}", witness_a, witness_b, actual_result, claimed_c)
                }
            });
            
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        },
        
        _ => {
            eprintln!("Unknown operation: {}", args[1]);
        }
    }
}

#[derive(Debug)]
struct ProofResult {
    proof_valid: bool,
    verification_passed: bool,
    setup_time: u64,
    index_time: u64,
    prove_time: u64,
    verify_time: u64,
}

fn run_full_proof_system(
    circuit: ConfigurableCircuit, 
    rng: &mut ChaCha20Rng
) -> Result<ProofResult, String> {
    
    // Step 1: Universal Setup
    println!("📱 第1步: 通用信任设置");
    let setup_start = Instant::now();
    let srs = MarlinInst::universal_setup(10, 10, 10, rng)
        .map_err(|e| format!("Universal setup failed: {:?}", e))?;
    let setup_time = setup_start.elapsed().as_millis() as u64;
    println!("✅ 通用信任设置完成 ({} ms)", setup_time);
    
    // Step 2: Index
    println!("📋 第2步: 电路索引");
    let index_start = Instant::now();
    let (pk, vk) = MarlinInst::index(&srs, circuit.clone())
        .map_err(|e| format!("Index failed: {:?}", e))?;
    let index_time = index_start.elapsed().as_millis() as u64;
    println!("✅ 电路索引完成 ({} ms)", index_time);
    
    // Step 3: Prove
    println!("🔒 第3步: 证明生成");
    let prove_start = Instant::now();
    let proof_result = MarlinInst::prove(&pk, circuit.clone(), rng);
    let prove_time = prove_start.elapsed().as_millis() as u64;
    
    let proof_valid = proof_result.is_ok();
    if proof_valid {
        println!("✅ 证明生成完成 ({} ms)", prove_time);
    } else {
        println!("❌ 证明生成失败 ({} ms): {:?}", prove_time, proof_result.as_ref().err());
    }
    
    // Step 4: Verify (only if proof generation succeeded)
    let (verification_passed, verify_time) = if let Ok(proof) = &proof_result {
        println!("✅ 第4步: 证明验证");
        let verify_start = Instant::now();
        
        let public_inputs = vec![circuit.public_c.unwrap()];
        let verify_result = MarlinInst::verify(&vk, &public_inputs, &proof, rng);
        let verify_time = verify_start.elapsed().as_millis() as u64;
        
        match verify_result {
            Ok(is_valid) => {
                if is_valid {
                    println!("✅ 证明验证完成 ({} ms) - 验证通过", verify_time);
                    (true, verify_time)
                } else {
                    println!("❌ 证明验证完成 ({} ms) - 验证失败", verify_time);
                    (false, verify_time)
                }
            },
            Err(e) => {
                println!("❌ 证明验证出错 ({} ms): {:?}", verify_time, e);
                (false, verify_time)
            }
        }
    } else {
        (false, 0)
    };
    
    println!("🎯 最终结果: 证明生成{}, 验证{}", 
        if proof_valid { "成功" } else { "失败" },
        if verification_passed { "通过" } else { "失败" }
    );
    
    Ok(ProofResult {
        proof_valid,
        verification_passed,
        setup_time,
        index_time,
        prove_time,
        verify_time,
    })
}