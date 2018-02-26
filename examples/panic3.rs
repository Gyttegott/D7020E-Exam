//! Minimal example with zero tasks
//#![deny(unsafe_code)]
// IMPORTANT always include this feature gate
#![feature(proc_macro)]
#![feature(used)]
#![no_std]

extern crate cortex_m_rtfm as rtfm;
// IMPORTANT always do this rename
extern crate stm32f413;

#[macro_use]
extern crate klee;
use klee::{k_abort, k_assert, k_assume};

// import the procedural macro
use rtfm::{app, Resource, Threshold};

app! {
    // this is the path to the device crate
    device: stm32f413,

    resources: {
        static X:u64 = 0;
    },

    tasks: {
        EXTI1: {
            path: exti1,
            priority: 1,
            resources: [X],
        },
        EXTI2: {
            path: exti2,
            priority: 1,
            resources: [X],
        },
    },
}

fn exti1(t: &mut Threshold, r: EXTI1::Resources) {
    k_assert(*r.X > 0);
}

fn exti2(t: &mut Threshold, mut r: EXTI2::Resources) {
    // k_assume(*r.X > 0 && *r.X < 8); // our pre-condition
    // let y = *r.X + 1;
    // *r.X = y;
    // k_assert(*r.X > 0 && *r.X < 8); // our post-condition
}
// The `init` function
//
// This function runs as an atomic section before any tasks
// are allowed to start
#[inline(never)]
#[allow(dead_code)]
fn init(_p: init::Peripherals) {}

// The idle loop.
//
// This runs after `init` and has a priority of 0. All tasks can preempt this
// function. This function can never return so it must contain some sort of
// endless loop.
#[inline(never)]
fn idle() -> ! {
    loop {}
}

// Assignments
//
// Rust RTFM is a framework for concurrent programming, ensuring race free
// access to shared resources. In this example we have a single resource X
// with a non atomic data type (u64), amounting to sequential reads
// (which are 32 bit on the ARM Cortex M).
//
// However the framework analysis is clever enought to realize that the tasks
// `EXTI1` and `EXTI2` can never preempt each other, hence we can
// can access the data without "claiming" the resouce.
//
// Access is done by "dereferencing" `*r.X`, and we can now assert the
// the value to be `*r.X`. However, as tasks operetate concurrently,
// (without knowlegde on other tasks in the system), our analysis here
// marks `X` as a (implicitly) symbolic.
//
// Compile and run the example in KLEE.
// Find the failing assertion, and the concrete assignment of X that
// triggers the fail.
//
// You might find that the task identifier is no longer 0.
// On the host side you find the list of tasks by.
//
// > more klee/tasks.txt
// autogenerated file
// ["EXTI1", "EXTI2"]
//
// In this case EXTI => task 0, EXTI2 = task 1, but they might be swapped
// due to the underlyind data structure being an (unordered) hash-map.
//
// Now uncomment the code in `exiti2` and comment out the assertion in `exti1`.
//
// Run the KLEE tool and find the failing assertion.
//
// What value of X was assigned by KLEE for the assertion to fail.
// ** your answer here **
//
// The designers intent was a saturating add (with a max value of 7)
//
// Fix the error by an if statement.
//
// Run KLEE and see that the analysis passes without errors.
//
// Show the lab assistant your solution.
//
// Comment out your solution.
//
// Now fix the error using the Rust `min` function (on u64).
//
// Run KLEE and see that the analysis passes without errors.
//
// Show the lab assistant your solution.
