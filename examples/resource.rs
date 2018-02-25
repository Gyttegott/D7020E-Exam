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
use klee::k_abort;

// import the procedural macro
use rtfm::{app, Resource, Threshold};

app! {
    // this is the path to the device crate
    device: stm32f413,

    resources: {
        static X:u32 = 0;
        static Y:u32 = 0;
    },

    tasks: {
        EXTI1: {
            path: exti1,
            priority: 1,
            resources: [X, Y],
        },

        EXTI2: {
            path: exti2,
            priority: 3,
            resources: [Y],
        },

        EXTI3: {
            path: exti3,
            priority: 2,
            resources: [X],
        },
    },
}

fn exti1(t: &mut Threshold, EXTI1::Resources { X, mut Y }: EXTI1::Resources) {
    X.claim(t, |x, t1| {
        Y.claim_mut(t1, |y, _| {
            if *x < 10 {
                for _ in 0..*x {
                    *y += 1;
                }
            }
        });
    });
}

fn exti2(t: &mut Threshold, mut r: EXTI2::Resources) {
    r.Y.claim_mut(t, |y, _| {
        if *y < 10 {
            *y += 1;
        } else {
            *y -= 1;
        }
    });
}

fn exti3(t: &mut Threshold, mut r: EXTI3::Resources) {
    r.X.claim_mut(t, |x, _| {
        *x += 1;
    });
}

#[inline(never)]
#[allow(dead_code)]
fn init(_p: init::Peripherals, _r: init::Resources) {
    loop {}
}

extern crate cortex_m;
use cortex_m::register::basepri;

// for wcet should be autogenerated...

#[inline(never)]
#[no_mangle]
fn readbasepri() -> u8 {
    cortex_m::register::basepri::read()
}
#[inline(never)]
#[allow(non_snake_case)]
#[no_mangle]
fn stub_EXTI1() {
    unsafe { _EXTI1() };
}
#[inline(never)]
#[no_mangle]
#[allow(non_snake_case)]
fn stub_EXTI2() {
    unsafe { _EXTI2() };
}
#[inline(never)]
#[no_mangle]
#[allow(non_snake_case)]
fn stub_EXTI3() {
    unsafe {
        _EXTI3();
    }
}

// The idle loop.
//
// This runs after `init` and has a priority of 0. All tasks can preempt this
// function. This function can never return so it must contain some sort of
// endless loop.
#[inline(never)]
fn idle() -> ! {
    readbasepri();
    stub_EXTI1();
    stub_EXTI2();
    stub_EXTI3();

    loop {
        rtfm::nop();
    }
}

//
//0x80001dc        0x80001dc <cortex_m_rt::reset_handler+4>
//0x8000270        0x8000270 <resource::init>
//0x8000284        0x8000284 <resource::idle+10>
