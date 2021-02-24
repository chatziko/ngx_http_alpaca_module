//! ALPaCA
//!
//! A library to implement the ALPaCA defense to Website Fingerprinting
//! attacks.
extern crate base64;
extern crate html5ever;
extern crate image;
extern crate kuchiki;
extern crate rand;
extern crate rand_distr;

pub mod aux;
pub mod deterministic;
pub mod distribution;
pub mod dom;
pub mod morphing;
pub mod pad;
