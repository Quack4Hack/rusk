// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;

use plonk_gadgets::AllocatedScalar;

// Prove that the amount inputted equals the amount outputted
pub fn sk_knowledge(composer: &mut StandardComposer, sk: AllocatedScalar, pk: AffinePoint) {
    
    let p1 = scalar_mul(composer, sk.var, GENERATOR_EXTENDED);

    composer.assert_equal_public_point(*p1.point(), pk);
    
}


#[cfg(test)]
mod commitment_tests {
    use super::*;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use rand::Rng;

    #[test]
    fn  sk_gadget() {
        
        let sk = JubJubScalar::random(&mut rand::thread_rng());
        let pk = AffinePoint::from(GENERATOR_EXTENDED * sk);
        
        

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 10, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 9).unwrap();
        let mut prover = Prover::new(b"test");

        let sk_r = AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(sk));

        sk_knowledge(prover.mut_cs(), sk_r, pk);

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");

        let sk_r = AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(sk));

        sk_knowledge(verifier.mut_cs(), sk_r, pk);
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}