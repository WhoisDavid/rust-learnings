# Rust learnings

## Projects

- [**Smart Pointers**](#smart-pointers)
- [**Procedural macros workshop**](#procedural-macros-workshop)
- [**Procedural macros playground**](#procedural-macros-playground)

## Smart pointers

Implementation of `std::cell`, `std::cell::RefCell` and `std::rc::Rc`.<br>
This was done following a great [Crust of Rust](https://www.youtube.com/playlist?list=PLqbS7AVVErFiWDOAVrPt7aYmnuuOLYvOa) on [Smart Pointers and Interior Mutability](https://youtu.be/8O0Nt9qY_vo) by Jon Gjengset ([@jonhoo](https://github.com/jonhoo)).

*Project under [smart-pointers](smart-pointers).*

## Procedural macros workshop

Started working on this [workshop for Rust Latam](https://github.com/dtolnay/proc-macro-workshop) by David Tolnay ([@dtolnay](https://github.com/dtolnay)).<br>
Also inspired by a long session on [proc macros](https://youtu.be/geovSK3wMB8) by Jon Gjengset ([@jonhoo](https://github.com/jonhoo))!

#### Progress
  - [x] [**Derive macro:** `derive(Builder)`](proc-macro-workshop/README.md#derive-macro-derivebuilder) 
    - Source: [proc-macro-workshop/builder/src/lib.rs](proc-macro-workshop/builder/src/lib.rs)
  - [ ] [**Derive macro:** `derive(CustomDebug)`](proc-macro-workshop/README.md#derive-macro-derivebuilder#derive-macro-derivecustomdebug)
  - [x] [**Function-like macro:** `seq!`](proc-macro-workshop/README.md#derive-macro-derivebuilder#function-like-macro-seq)
    - Source: [proc-macro-workshop/seq-impl/src/lib.rs](proc-macro-workshop/seq-impl/src/lib.rs)
  - [ ] [**Attribute macro:** `#[sorted]`](proc-macro-workshop/README.md#derive-macro-derivebuilder#attribute-macro-sorted)
  - [ ] [**Attribute macro:** `#[bitfield]`](proc-macro-workshop/README.md#derive-macro-derivebuilder#attribute-macro-bitfield)


*Project under [proc-macro-workshop](proc-macro-workshop).*

## Procedural macros playground

Toy repo to do some tests on proc macro syntax.

*Project under [proc-macro-playground](proc-macro-playground).*

