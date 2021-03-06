//! Minimal example with zero tasks
//#![deny(unsafe_code)]
// IMPORTANT always include this feature gate
#![feature(proc_macro)]
#![no_std]

extern crate cortex_m_rtfm as rtfm;
// IMPORTANT always do this rename
extern crate stm32f103xx;

#[macro_use]
extern crate klee;
use klee::{k_abort, k_assert};

// import the procedural macro
use rtfm::app;

app! {
    // this is the path to the device crate
    device: stm32f103xx,
}

//#[inline(never)]
fn t() -> i32 {
    let mut x = 0;
    let x: &mut i32 = &mut x;
    let mut y = 0;
    k_symbol!(x, "x");
    if *x < 10 {
        for _ in 0..*x {
            y += 1;
        }
    }
    y
}

#[inline(never)]
fn init(_p: init::Peripherals) {
    t();
}

// The idle loop.
//
// This runs after `init` and has a priority of 0. All tasks can preempt this
// function. This function can never return so it must contain some sort of
// endless loop.

// #[inline(never)]
// fn idle() -> ! {
//     k_abort();
// }
