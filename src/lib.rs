#[macro_use]
extern crate nom;
#[macro_use]
extern crate serde_derive;
extern crate petgraph;

pub mod codegen;
pub mod dependencies;
pub mod ir;
pub mod vectorization;
