use ark_bls12_381::{Bls12_381, Fr};

use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_poly::univariate::DensePolynomial;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError};
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

// Simple multiplication circuit: a * b = c
#[derive(Clone)]
struct MultiplicationCircuit {
    a: Option<Fr>,
    b: Option<Fr>,
    c: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for MultiplicationCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = cs.new_input_variable(|| self.c.ok_or(SynthesisError::AssignmentMissing))?;

        cs.enforce_constraint(
            lc!() + a,
            lc!() + b, 
            lc!() + c
        )?;

        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <operation> [args...]", args[0]);
        eprintln!("Operations:");
        eprintln!("  universal_setup <num_constraints> <num_variables> <num_non_zero>");
        eprintln!("  index <circuit_name> <a> <b> <c>");
        eprintln!("  prove <witness_a> <witness_b> <public_c>");
        eprintln!("  verify <public_c>");
        eprintln!("  full_demo <a> <b> <c>");
        return;
    }

    let mut rng = ChaCha20Rng::seed_from_u64(0u64);
    
    match args[1].as_str() {
        "universal_setup" => {
            if args.len() != 5 {
                eprintln!("Usage: {} universal_setup <num_constraints> <num_variables> <num_non_zero>", args[0]);
                return;
            }
            
            let num_constraints: usize = args[2].parse().unwrap();
            let num_variables: usize = args[3].parse().unwrap();
            let num_non_zero: usize = args[4].parse().unwrap();
            
            println!("🔧 开始通用信任设置...");
            let start = Instant::now();
            
            match MarlinInst::universal_setup(num_constraints, num_variables, num_non_zero, &mut rng) {
                Ok(_srs) => {
                    let duration = start.elapsed();
                    let result = serde_json::json!({
                        "success": true,
                        "message": format!("Universal setup completed successfully"),
                        "max_degree": num_constraints + num_variables,
                        "setup_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                },
                Err(e) => {
                    let duration = start.elapsed();
                    let result = serde_json::json!({
                        "success": false,
                        "message": format!("Universal setup failed: {:?}", e),
                        "setup_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
            }
        },
        
        "index" => {
            if args.len() != 6 {
                eprintln!("Usage: {} index <circuit_name> <a> <b> <c>", args[0]);
                return;
            }
            
            let circuit_name = &args[2];
            let a: u64 = args[3].parse().unwrap();
            let b: u64 = args[4].parse().unwrap();
            let c: u64 = args[5].parse().unwrap();
            
            println!("📋 开始电路索引生成...");
            let start = Instant::now();
            
            // First setup
            let srs = match MarlinInst::universal_setup(10, 10, 10, &mut rng) {
                Ok(srs) => srs,
                Err(e) => {
                    let result = serde_json::json!({
                        "success": false,
                        "message": format!("Setup failed: {:?}", e),
                        "index_time": 0
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    return;
                }
            };
            
            let circuit = MultiplicationCircuit {
                a: Some(Fr::from(a)),
                b: Some(Fr::from(b)),
                c: Some(Fr::from(c)),
            };

            match MarlinInst::index(&srs, circuit.clone()) {
                Ok((_pk, _vk)) => {
                    let duration = start.elapsed();
                    
                    // Get constraint system info
                    let cs = ConstraintSystem::new_ref();
                    let _ = circuit.generate_constraints(cs.clone());
                    let num_constraints = cs.num_constraints();
                    let num_variables = cs.num_instance_variables() + cs.num_witness_variables();
                    
                    let result = serde_json::json!({
                        "success": true,
                        "message": format!("Circuit '{}' indexed successfully", circuit_name),
                        "num_constraints": num_constraints,
                        "num_variables": num_variables,
                        "index_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                },
                Err(e) => {
                    let duration = start.elapsed();
                    let result = serde_json::json!({
                        "success": false,
                        "message": format!("Circuit indexing failed: {:?}", e),
                        "index_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
            }
        },
        
        "prove" => {
            if args.len() != 5 {
                eprintln!("Usage: {} prove <witness_a> <witness_b> <public_c>", args[0]);
                return;
            }
            
            let witness_a: u64 = args[2].parse().unwrap();
            let witness_b: u64 = args[3].parse().unwrap();
            let public_c: u64 = args[4].parse().unwrap();
            
            println!("🔒 开始证明生成...");
            let start = Instant::now();
            
            // Setup and index
            let srs = MarlinInst::universal_setup(10, 10, 10, &mut rng).unwrap();
            let circuit = MultiplicationCircuit {
                a: Some(Fr::from(witness_a)),
                b: Some(Fr::from(witness_b)),
                c: Some(Fr::from(public_c)),
            };
            let (pk, _vk) = MarlinInst::index(&srs, circuit.clone()).unwrap();

            match MarlinInst::prove(&pk, circuit, &mut rng) {
                Ok(_proof) => {
                    let duration = start.elapsed();
                    let result = serde_json::json!({
                        "success": true,
                        "message": format!("Proof generated successfully for {}×{}={}", witness_a, witness_b, public_c),
                        "proof_size": 1000, // Rough estimate
                        "proof_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                },
                Err(e) => {
                    let duration = start.elapsed();
                    let result = serde_json::json!({
                        "success": false,
                        "message": format!("Proof generation failed: {:?}", e),
                        "proof_time": duration.as_millis()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
            }
        },
        
        "verify" => {
            if args.len() != 3 {
                eprintln!("Usage: {} verify <public_c>", args[0]);
                return;
            }
            
            let public_c: u64 = args[2].parse().unwrap();
            
            println!("✅ 开始证明验证...");
            let start = Instant::now();
            
            // Setup, index and prove
            let srs = MarlinInst::universal_setup(10, 10, 10, &mut rng).unwrap();
            let circuit = MultiplicationCircuit {
                a: Some(Fr::from(3u64)),
                b: Some(Fr::from(5u64)),
                c: Some(Fr::from(public_c)),
            };
            let (pk, vk) = MarlinInst::index(&srs, circuit.clone()).unwrap();
            
            match MarlinInst::prove(&pk, circuit, &mut rng) {
                Ok(proof) => {
                    let public_inputs = vec![Fr::from(public_c)];
                    match MarlinInst::verify(&vk, &public_inputs, &proof, &mut rng) {
                        Ok(is_valid) => {
                            let duration = start.elapsed();
                            let result = serde_json::json!({
                                "success": true,
                                "message": if is_valid { 
                                    "Proof verification successful - the constraint is satisfied!" 
                                } else { 
                                    "Proof verification failed - the constraint is not satisfied!" 
                                },
                                "is_valid": is_valid,
                                "verify_time": duration.as_millis()
                            });
                            println!("{}", serde_json::to_string_pretty(&result).unwrap());
                        },
                        Err(e) => {
                            let duration = start.elapsed();
                            let result = serde_json::json!({
                                "success": false,
                                "message": format!("Verification failed: {:?}", e),
                                "is_valid": false,
                                "verify_time": duration.as_millis()
                            });
                            println!("{}", serde_json::to_string_pretty(&result).unwrap());
                        }
                    }
                },
                Err(e) => {
                    let result = serde_json::json!({
                        "success": false,
                        "message": format!("Could not generate proof for verification: {:?}", e),
                        "is_valid": false,
                        "verify_time": 0
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap());
                }
            }
        },
        
        "full_demo" => {
            if args.len() != 5 {
                eprintln!("Usage: {} full_demo <a> <b> <c>", args[0]);
                return;
            }
            
            let a: u64 = args[2].parse().unwrap();
            let b: u64 = args[3].parse().unwrap();
            let c: u64 = args[4].parse().unwrap();
            
            println!("🎬 开始完整的零知识证明演示...");
            
            // Step 1: Universal Setup
            println!("\n📱 第1步: 通用信任设置");
            let setup_start = Instant::now();
            let srs = match MarlinInst::universal_setup(10, 10, 10, &mut rng) {
                Ok(srs) => {
                    println!("✅ 通用信任设置完成 ({} ms)", setup_start.elapsed().as_millis());
                    srs
                },
                Err(e) => {
                    println!("❌ 通用信任设置失败: {:?}", e);
                    return;
                }
            };
            
            // Step 2: Index
            println!("\n📋 第2步: 电路索引");
            let index_start = Instant::now();
            let circuit = MultiplicationCircuit {
                a: Some(Fr::from(a)),
                b: Some(Fr::from(b)),
                c: Some(Fr::from(c)),
            };
            let (pk, vk) = match MarlinInst::index(&srs, circuit.clone()) {
                Ok((pk, vk)) => {
                    println!("✅ 电路索引完成 ({} ms)", index_start.elapsed().as_millis());
                    (pk, vk)
                },
                Err(e) => {
                    println!("❌ 电路索引失败: {:?}", e);
                    return;
                }
            };
            
            // Step 3: Prove
            println!("\n🔒 第3步: 证明生成");
            let prove_start = Instant::now();
            let proof = match MarlinInst::prove(&pk, circuit, &mut rng) {
                Ok(proof) => {
                    println!("✅ 证明生成完成 ({} ms)", prove_start.elapsed().as_millis());
                    proof
                },
                Err(e) => {
                    println!("❌ 证明生成失败: {:?}", e);
                    return;
                }
            };
            
            // Step 4: Verify
            println!("\n✅ 第4步: 证明验证");
            let verify_start = Instant::now();
            let public_inputs = vec![Fr::from(c)];
            match MarlinInst::verify(&vk, &public_inputs, &proof, &mut rng) {
                Ok(is_valid) => {
                    println!("✅ 证明验证完成 ({} ms)", verify_start.elapsed().as_millis());
                    println!("🎯 验证结果: {}", if is_valid { "成功" } else { "失败" });
                    
                    let total_result = serde_json::json!({
                        "demo_complete": true,
                        "constraint": format!("{} × {} = {}", a, b, c),
                        "is_constraint_satisfied": a * b == c,
                        "proof_verification": is_valid,
                        "timing": {
                            "setup_ms": setup_start.elapsed().as_millis(),
                            "index_ms": index_start.elapsed().as_millis(),
                            "prove_ms": prove_start.elapsed().as_millis(),
                            "verify_ms": verify_start.elapsed().as_millis()
                        }
                    });
                    println!("\n📊 演示结果:");
                    println!("{}", serde_json::to_string_pretty(&total_result).unwrap());
                },
                Err(e) => {
                    println!("❌ 证明验证失败: {:?}", e);
                }
            }
        },
        
        _ => {
            eprintln!("Unknown operation: {}", args[1]);
        }
    }
}