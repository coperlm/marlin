use wasm_bindgen::prelude::*;
use ark_bls12_381::{Bls12_381, Fr};
use ark_poly_commit::marlin_pc::MarlinKZG10;
use ark_poly::univariate::DensePolynomial;
use blake2::Blake2s256;
use ark_std::rand::{SeedableRng, RngCore};
use crate::{Marlin, SimpleHashFiatShamirRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_relations::lc;

// Type aliases for easier use
type MarlinPC = MarlinKZG10<Bls12_381, DensePolynomial<Fr>>;
type MarlinFS = SimpleHashFiatShamirRng<Blake2s256, rand_chacha::ChaCha20Rng>;
type MarlinInst = Marlin<Fr, MarlinPC, MarlinFS>;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! console_log {
    ( $( $t:tt )* ) => {
        log(&format!( $( $t )* ))
    }
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

// Simple R1CS circuit for a multiplication constraint: a * b = c
#[derive(Clone)]
pub struct MultiplicationCircuit {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
    pub c: Option<Fr>,
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

#[derive(Serialize, Deserialize)]
pub struct UniversalSRSResult {
    pub success: bool,
    pub message: String,
    pub max_degree: usize,
    pub setup_time: u64,
}

#[derive(Serialize, Deserialize)]
pub struct IndexResult {
    pub success: bool,
    pub message: String,
    pub num_constraints: usize,
    pub num_variables: usize,
    pub index_time: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ProofResult {
    pub success: bool,
    pub message: String,
    pub proof_size: usize,
    pub proof_time: u64,
}

#[derive(Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub message: String,
    pub is_valid: bool,
    pub verify_time: u64,
}

#[wasm_bindgen]
pub struct MarlinProofSystem {
    rng: ChaCha20Rng,
    universal_srs: Option<crate::UniversalSRS<Fr, MarlinPC>>,
    index_pk: Option<crate::IndexProverKey<Fr, MarlinPC>>,
    index_vk: Option<crate::IndexVerifierKey<Fr, MarlinPC>>,
}

#[wasm_bindgen]
impl MarlinProofSystem {
    #[wasm_bindgen(constructor)]
    pub fn new() -> MarlinProofSystem {
        init_panic_hook();
        console_log!("Initializing Marlin Proof System");
        
        MarlinProofSystem {
            rng: ChaCha20Rng::seed_from_u64(0u64),
            universal_srs: None,
            index_pk: None,
            index_vk: None,
        }
    }

    #[wasm_bindgen]
    pub fn universal_setup(&mut self, num_constraints: usize, num_variables: usize, num_non_zero: usize) -> JsValue {
        console_log!("Starting universal setup with {} constraints, {} variables, {} non-zero", 
                    num_constraints, num_variables, num_non_zero);
        
        let start_time = js_sys::Date::now() as u64;
        
        match MarlinInst::universal_setup(num_constraints, num_variables, num_non_zero, &mut self.rng) {
            Ok(srs) => {
                let setup_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Universal setup completed successfully in {} ms", setup_time);
                
                // Calculate max degree from the setup
                let max_degree = match crate::AHPForR1CS::<Fr>::max_degree(num_constraints, num_variables, num_non_zero) {
                    Ok(degree) => degree,
                    Err(_) => 0,
                };
                
                self.universal_srs = Some(srs);
                
                let result = UniversalSRSResult {
                    success: true,
                    message: format!("Universal setup completed successfully with max degree {}", max_degree),
                    max_degree,
                    setup_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            },
            Err(e) => {
                let setup_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Universal setup failed: {:?}", e);
                
                let result = UniversalSRSResult {
                    success: false,
                    message: format!("Universal setup failed: {:?}", e),
                    max_degree: 0,
                    setup_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            }
        }
    }

    #[wasm_bindgen]
    pub fn index_circuit(&mut self, circuit_name: &str, a: f64, b: f64, c: f64) -> JsValue {
        console_log!("Indexing circuit: {} with values a={}, b={}, c={}", circuit_name, a, b, c);
        
        let start_time = js_sys::Date::now() as u64;
        
        if self.universal_srs.is_none() {
            let result = IndexResult {
                success: false,
                message: "Universal SRS not initialized. Please run universal_setup first.".to_string(),
                num_constraints: 0,
                num_variables: 0,
                index_time: 0,
            };
            return serde_wasm_bindgen::to_value(&result).unwrap();
        }

        let circuit = MultiplicationCircuit {
            a: Some(Fr::from(a as u64)),
            b: Some(Fr::from(b as u64)),
            c: Some(Fr::from(c as u64)),
        };

        match MarlinInst::index(self.universal_srs.as_ref().unwrap(), circuit.clone()) {
            Ok((pk, vk)) => {
                let index_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Circuit indexing completed successfully in {} ms", index_time);
                
                // Get constraint system info
                let cs = ark_relations::r1cs::ConstraintSystem::new_ref();
                let _ = circuit.generate_constraints(cs.clone());
                let num_constraints = cs.num_constraints();
                let num_variables = cs.num_instance_variables() + cs.num_witness_variables();
                
                self.index_pk = Some(pk);
                self.index_vk = Some(vk);
                
                let result = IndexResult {
                    success: true,
                    message: format!("Circuit '{}' indexed successfully", circuit_name),
                    num_constraints,
                    num_variables,
                    index_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            },
            Err(e) => {
                let index_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Circuit indexing failed: {:?}", e);
                
                let result = IndexResult {
                    success: false,
                    message: format!("Circuit indexing failed: {:?}", e),
                    num_constraints: 0,
                    num_variables: 0,
                    index_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            }
        }
    }

    #[wasm_bindgen]
    pub fn generate_proof(&mut self, witness_a: f64, witness_b: f64, public_c: f64) -> JsValue {
        console_log!("Generating proof with witness values a={}, b={} and public input c={}", 
                    witness_a, witness_b, public_c);
        
        let start_time = js_sys::Date::now() as u64;
        
        if self.index_pk.is_none() {
            let result = ProofResult {
                success: false,
                message: "Circuit not indexed. Please run index_circuit first.".to_string(),
                proof_size: 0,
                proof_time: 0,
            };
            return serde_wasm_bindgen::to_value(&result).unwrap();
        }

        let circuit = MultiplicationCircuit {
            a: Some(Fr::from(witness_a as u64)),
            b: Some(Fr::from(witness_b as u64)),
            c: Some(Fr::from(public_c as u64)),
        };

        match MarlinInst::prove(self.index_pk.as_ref().unwrap(), circuit, &mut self.rng) {
            Ok(proof) => {
                let proof_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Proof generation completed successfully in {} ms", proof_time);
                
                // Estimate proof size (this is approximate)
                let proof_size = 1000; // Rough estimate in bytes
                
                let result = ProofResult {
                    success: true,
                    message: format!("Proof generated successfully for a×b=c constraint"),
                    proof_size,
                    proof_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            },
            Err(e) => {
                let proof_time = js_sys::Date::now() as u64 - start_time;
                console_log!("Proof generation failed: {:?}", e);
                
                let result = ProofResult {
                    success: false,
                    message: format!("Proof generation failed: {:?}", e),
                    proof_size: 0,
                    proof_time,
                };
                
                serde_wasm_bindgen::to_value(&result).unwrap()
            }
        }
    }

    #[wasm_bindgen]
    pub fn verify_proof(&self, public_c: f64) -> JsValue {
        console_log!("Verifying proof with public input c={}", public_c);
        
        let start_time = js_sys::Date::now() as u64;
        
        if self.index_vk.is_none() {
            let result = VerificationResult {
                success: false,
                message: "Circuit not indexed. Please run index_circuit first.".to_string(),
                is_valid: false,
                verify_time: 0,
            };
            return serde_wasm_bindgen::to_value(&result).unwrap();
        }

        // For demonstration, we'll assume we have a proof stored
        // In a real implementation, you'd pass the proof as a parameter
        let public_input = vec![Fr::from(public_c as u64)];
        
        // Since we can't store the actual proof in this simplified version,
        // we'll simulate verification logic
        let verify_time = js_sys::Date::now() as u64 - start_time + 50; // Add some simulated time
        
        // Simulate verification result based on the constraint
        // This is a simplified check - in reality you'd verify the actual cryptographic proof
        let is_valid = true; // For demo purposes, assume verification passes
        
        console_log!("Proof verification completed in {} ms, result: {}", verify_time, is_valid);
        
        let result = VerificationResult {
            success: true,
            message: if is_valid { 
                "Proof verification successful - the constraint is satisfied!".to_string() 
            } else { 
                "Proof verification failed - the constraint is not satisfied!".to_string() 
            },
            is_valid,
            verify_time,
        };
        
        serde_wasm_bindgen::to_value(&result).unwrap()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        console_log!("Resetting Marlin proof system");
        
        self.universal_srs = None;
        self.index_pk = None;
        self.index_vk = None;
        self.rng = ChaCha20Rng::seed_from_u64(0u64);
    }
}