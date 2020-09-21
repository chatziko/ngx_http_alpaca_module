//! ALPaCA
//!
//! A library to implement the ALPaCA defense to Website Fingerprinting
//! attacks.
extern crate rand;
extern crate rand_distr;
extern crate html5ever;
extern crate kuchiki;

pub mod pad;
pub mod dom;
pub mod morphing;
pub mod distribution;
pub mod deterministic;
pub mod aux;
