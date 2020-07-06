# Rust learnings

## Projects

- [**Smart Pointers**](#smart-pointers)
- [**Procedural macros workshop**](#procedural-macros-workshop)
- [**Procedural macros playground**](#procedural-macros-playground)

## Smart pointers

Implementation of [`std::cell`](smart-pointers/src/cell.rs), [`std::cell::RefCell`](smart-pointers/src/refcell.rs) and [`std::rc::Rc`](smart-pointers/src/rc.rs).<br>
This was done following a great [Crust of Rust](https://www.youtube.com/playlist?list=PLqbS7AVVErFiWDOAVrPt7aYmnuuOLYvOa) on [Smart Pointers and Interior Mutability](https://youtu.be/8O0Nt9qY_vo) by Jon Gjengset ([@jonhoo](https://github.com/jonhoo)).

*Project under [smart-pointers](smart-pointers).*

## Procedural macros workshop

Started working on this [workshop for Rust Latam](https://github.com/dtolnay/proc-macro-workshop) by David Tolnay ([@dtolnay](https://github.com/dtolnay)).<br>
Also inspired by a long session on [proc macros](https://youtu.be/geovSK3wMB8) by Jon Gjengset ([@jonhoo](https://github.com/jonhoo))!

#### Progress
  - [x] [**Derive macro:** `derive(Builder)`](proc-macro-workshop/README.md#derive-macro-derivebuilder) 
    - Source: [proc-macro-workshop/builder/src/lib.rs](proc-macro-workshop/builder/src/lib.rs)
  - [x] [**Derive macro:** `derive(CustomDebug)`](proc-macro-workshop/README.md#derive-macro-derivebuilder#derive-macro-derivecustomdebug)
    - Source: [proc-macro-workshop/debug/src/lib.rs](proc-macro-workshop/debug/src/lib.rs)
  - [x] [**Function-like macro:** `seq!`](proc-macro-workshop/README.md#function-like-macro-seq)
    - Source: [proc-macro-workshop/seq-impl/src/lib.rs](proc-macro-workshop/seq-impl/src/lib.rs)
  - [x] [**Attribute macro:** `#[sorted]`](proc-macro-workshop/README.md#attribute-macro-sorted)
    - Source: [proc-macro-workshop/sorted/src/lib.rs](proc-macro-workshop/sorted/src/lib.rs)
  - [ ] [**Attribute macro:** `#[bitfield]`](proc-macro-workshop/README.md#attribute-macro-bitfield)


*Project under [proc-macro-workshop](proc-macro-workshop).*

## Procedural macros playground

Toy repo to do some tests on proc macro syntax.

*Project under [proc-macro-playground](proc-macro-playground).*

