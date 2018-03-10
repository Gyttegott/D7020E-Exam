//! Example using shared resources
// IMPORTANT always include this feature gate
#![feature(proc_macro)]
#![feature(used)]
#![no_std]
#![feature(asm)]
extern crate cortex_m_rtfm as rtfm;
// IMPORTANT always do this rename
extern crate stm32f413;

#[macro_use]
extern crate klee;
use klee::*;
// use rtfm::{bkpt_1, bkpt_2, bkpt_3};

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

#[allow(non_snake_case)]
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

#[allow(non_snake_case)]
fn exti2(t: &mut Threshold, mut r: EXTI2::Resources) {
    r.Y.claim_mut(t, |y, _| {
        if *y < 10 {
            *y += 1;
        } else {
            *y -= 1;
        }
    });
}

#[allow(non_snake_case)]
fn exti3(t: &mut Threshold, mut r: EXTI3::Resources) {
    r.X.claim_mut(t, |x, _| {
        *x += 1;
    });
}

#[inline(never)]
#[allow(dead_code)]
fn init(_p: init::Peripherals, _r: init::Resources) {}

#[inline(never)]
#[allow(dead_code)]
fn idle() -> ! {
    loop {
        rtfm::nop();
    }
}

// Static code analysis
// In this lab we will study how a Rust RTFM application can be
// statically analysed.
//
// The application has three tasks EXTI1, EXTI2 and EXTI3,
// with associated priorites 1 (lowest), 3 (highest), and 2 (mid).
//
// Assignment 1.
// Build the application for KLEE analysis.
// > xargo build --example resource  --features klee_mode --target x86_64-unknown-linux-gnu
//
// Start the KLEE docker
// > docker run --rm --user $(id -u):$(id -g) -v $PWD/target/x86_64-unknown-linux-gnu/debug/examples:/mnt -w /mnt -it afoht/llvm-klee-4 /bin/bash
//
// Notice, in this case we have compiled in dev/debug mode, such to avoid optimzation
// of the generated LLVM IR code. Thus the docker should run in the `.../debug/..` folder.
//
// Now run KLEE in the docker.
// > klee resource-*.bc
// KLEE: WARNING: executable has module level assembly (ignoring)
// KLEE: ERROR: /home/pln/.cargo/registry/src/github.com-1ecc6299db9ec823/cortex-m-rt-0.3.13/src/lang_items.rs:1: abort failure
// KLEE: NOTE: now ignoring this error at this location

// KLEE: done: total instructions = 11575
// KLEE: done: completed paths = 25
// KLEE: done: generated tests = 16
//
// When compiled with the --features klee_mode, resources are treated as symbolic.
// Investigate the output files (in `klee-last`). Use `ktest-tool` to examine the tests.
// The file `klee/tasks.txt` gives the order of tasks, it may vary for each run.
// ```
// // autogenerated file
// ["EXTI1", "EXTI2", "EXTI3"]
// ```
//
// How many tests were generated for task EXIT1?
// ** your answer here **
// Why that many, (look at the code and the generted tests).
// ** your answer here **
//
// How many tests were generated for task EXIT2?
// ** your answer here **
// Why that many, (look at the code and the generted tests).
// ** your answer here **
//
// How many tests were generated for task EXIT3?
// ** your answer here **
// Why that many, (look at the code and the generted tests).
// ** your answer here **
//
// (There will be one additional "dymmy" test for testing "no task".)
//
// Now identify the cause of generated error.
// > more klee-last/*.abort.err
//
// Find the error on the source code and fix it.
// Hint, use *wrapping arithmetics*.
//
// Repeat the code generation, run KLEE agian and fix errors until
// no more errors are found.
//
// How many errors were identified?
// ** your answer here **
//
// For one of the arithmetic operations, no error was found.
// Explain why this is no error.
// ** your answer here **
//
// Why didn't KLEE report all the errors initially?
// Scroll back and compare the stack traces of the generated errors.
// Hint, for each line in the code KLEE will spot ONE error.
// ** your answer here **
//
// Assignment 2.
// KLEE has now generated tests covering EACH feasible path for EACH task.
// Now we will analyse the exection time for each such test.
//
// In order to do that comiple your application.
// > xargo build --example resource --release --features wcet_bkpt --target thumbv7em-none-eabihf
//
// We build in --release for optimizing the performance.
// The --features wcet_bkpt will insert a `bkpt` instruction on LOCK ond UNLOCK of
// of each resource. This allows you to monitor the number of clock cyckles using gdb.
//
// Connect your Nucleo64 board with the `stm32f401re` (or `stm32f411re`) MCU.
// Start openocd in a separate terminal (and let it run there).
//
// > openocd -f interface/stlink.cfg -f target/stm32f4x.cfg
// (You may use `-f inteface/stlink-v2-1.cfg`, if `stlink.cfg` is missing.)
//
// Start gdb.
// > arm-none-eabi-gdb -x gdbinit_manual target/thumbv7em-none-eabihf/release/examples/resource
//
// The `gdbinit_manual` has the following content.
// ```
// target remote :3333
// mon reset halt
// load
// tb main
// continue
//
// Line by line, it:
// - connects to the target (the MCU)
// - resets the MCU
// - loads the binary, in this case set to target/thumbv7em-none-eabihf/release/examples/resource
// - sets a temporary breakpoint to `main`, and
// - continues executing until `main` is reached.
//
// At the point the MCU hits `main` it stops.
// (You may be promted to continue working <return> or quit, in such case press <return>.)
//
// Now you can inspect where the processor halted.
// >  (gdb) list
// 560                 // be executed *before* enabling the FPU and that would generate an
// 561                 // exception
// 562                 #[inline(never)]
// 563                 fn main() {
// 564                     unsafe {
// 565                         ::main(0, ::core::ptr::null());
// 566                     }
// 567                 }
// 568
// 569                 main()
//
// As you see this is not part of your program, rather it originates from the library startup code.
//
// This is perfect, now the MCU is initiated and under your control.
//
// At this point the memory (resources), holds their initial values from the
// your application (this was set by the library code prior to hitting main).
// You can inspect the global reasources (they are prepended by `_`).
//
// (gdb) p resource::_X
// $1 = 0
// (gdb) p resource::_Y
// $2 = 0
//
// Before calling our tasks we need to enable the debug timer (cycle counter in the DWT unit of the MCU).
//
// (gdb) mon mww 0xe0001000 1
//
// and set the counter value to 0
//
// (gdb) mon mww 0xe0001004 0
//
// Now we can call the task(s). Lets start easy with EXTI3.
// (gdb) call stub_EXTI3()
// (gdb) mon mrw 0xe0001004
// 9
//
// Wow, it took only 9 clock cycles to run the complete task!!!!
// (Compare to a threaded OS that would require several 100 clock cycles to start a thread/task)
//
// But hey, where did my claim go? Wasn't I supposed to hit a breakpoint when claiming the resource?
// (In this case the resource X)
// Well Rust RTFM is smart enough to know that you are already at sufficient threshold (priority)
// in order to grant you the resource dircectly. Let us look at the code.
//
// (gdb) disassemble stub_EXTI3
// Dump of assembler code for function stub_EXTI3:
//    0x08000268 <+0>:     movw    r0, #0
//    0x0800026c <+4>:     movt    r0, #8192       ; 0x2000
//    0x08000270 <+8>:     ldr     r1, [r0, #0]
//    0x08000272 <+10>:    adds    r1, #1
//    0x08000274 <+12>:    str     r1, [r0, #0]
//    0x08000276 <+14>:    bx      lr
// End of assembler dump.
//
// Here you go, cannot be simpler that this, it:
// - sets up the address to X in r0
// - reads the old value
// - adds 1 (wrapping arithmetics)
// - stores the new value in X, and
// - it returns
// (On Cortex M functions and inerrupt handlers are essentially treated the same way)
//
// So now you can check the value of X. See above...
// ** your answer here **
//
// Now let us turn to EXTI2.
// (gdb) mon mww 0xe0001004 0
// (gdb) call stub_EXTI2()
// (gdb) mon mrw 0xe0001004
// 13
//
// Also in this case we see that the resource (`Y`) was granted without overhead
// The code took a few additional clock cycles (its contians a condition, right).
// Figure out which path was triggered by this test.
// ** your answer here **
//
// Now figure out how to test the other branch
// you can use
// (gdb) set resource::_Y=...
// to set a value in memory
//
// Make a call to the `stub_EXTI2()` with the set value of `_Y`.
// How many clock cycles did you get (rember to set the counter to 0 before calling)
// ** your answer here **
//
// Check the nwe value of `_Y`.
// ** your answer here **
//
// Hint, if done correctly, you should have the same number of cycles for both paths.
// Ok, so the cycle count was the same ;)
// How so, lets have a look at the generated assembly.
//
//  (gdb) disassemble stub_EXTI2
// Dump of assembler code for function stub_EXTI2:
//    0x080002aa <+0>:     movw    r0, #0
//    0x080002ae <+4>:     mov.w   r2, #4294967295 ; 0xffffffff
//    0x080002b2 <+8>:     movt    r0, #8192       ; 0x2000
//    0x080002b6 <+12>:    ldr     r1, [r0, #4]
//    0x080002b8 <+14>:    cmp     r1, #10
//    0x080002ba <+16>:    it      cc
//    0x080002bc <+18>:    movcc   r2, #1
//    0x080002be <+20>:    add     r1, r2
//    0x080002c0 <+22>:    str     r1, [r0, #4]
//    0x080002c2 <+24>:    bx      lr
// End of assembler dump.
//
// This one is a bit harder to understand.
// What it does essentially is:
// - read `Y` into r1,
// - store -1 in r2
// - conditionally overwrite the r2 by a 1 if `Y < 10`
// - add r2 to r1 (the value of `Y`),
// - update `Y`.
//
// So this is beautiful, the COSTLY branch is replaced by a CHEAP
// condtitonal assigmnet `movcc`.
// You can read more on this on
// http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.dui0553a/BABEHFEF.html
//
// Ok, back to the program...
// Now recall the KLEE generated test cases.
// Spot the ones regarding EXTI2.
// What assignments would KLEE suggest to `Y` to test both paths.
// ** your answers here **
//
// So, your test assignments or the ones from KLEE, which ones is better?
// ** your answer here **

// (We will return to optimising test cases later... so put this on your mental stack :)
//
// Now lets have a look at the tricky one EXTI1.
// (gdb) mon mww 0xe0001004 0
// (gdb) call stub_EXTI1()
// Program received signal SIGTRAP, Trace/breakpoint trap.
// cortex_m_rtfm::claim (ceiling=2, _nvic_prio_bits=4, data=<optimized out>, t=<optimized out>, f=...)
//     at /home/pln/klee/cortex-m-rtfm/src/lib.rs:167
// 167                             bkpt();
// The program being debugged was signaled while in a function called from GDB.
// GDB remains in the frame where the signal was received.
// To change this behavior use "set unwindonsignal on".
// Evaluation of the expression containing the function
// (stub_EXTI1) will be abandoned.
// When the function is done executing, GDB will silently stop.
//
// So what is happening here is that it runs into the (first) brkp intstruction (LOCK `X`).
// You may ignore the verbose output at this point....
//
// Run
// (gdb) mon mrw 0xe0001004
// 15
//
// So all in all it took 15 cycles to get there, `claim X`.
// We can look at the code causing the `bkpt`
// (gdb) list
// 162                         }
// 163
// 164                         // wcet_bkpt mode
// 165                         // put breakpoint at raise ceiling, for tracing execution time
// 166                         if cfg!(feature = "wcet_bkpt") {
// 167                             bkpt();
// 168                         }
//
// That makes sense, we are in the RTFM library handling the `claim X`
// (gdb) c
// Continuing.
// Program received signal SIGTRAP, Trace/breakpoint trap.
// cortex_m_rtfm::claim (ceiling=3, _nvic_prio_bits=4, data=<optimized out>, t=<optimized out>, f=...)
//     at /home/pln/klee/cortex-m-rtfm/src/lib.rs:167
// 167                             bkpt();
// (gdb) mon mrw 0xe0001004
// 19
//
// So after 19 clock cycles we claim `Y`
//
// Now lets have a look at the generated assembly.
// (gdb) disassemble
// ...
//    0x080002f6 <+4>:     bcs.n   0x8000334 <resource::<impl rtfm_core::Resource for resource::EXTI1::X>::claim+66>
//    0x080002f8 <+6>:     movs    r1, #224        ; 0xe0
//    0x080002fa <+8>:     movs    r2, #208        ; 0xd0
//    0x080002fc <+10>:    mrs     r12, BASEPRI
//    0x08000300 <+14>:    msr     BASEPRI, r1
//    0x08000304 <+18>:    bkpt    0x0000
//    0x08000306 <+20>:    mrs     r1, BASEPRI
//    0x0800030a <+24>:    msr     BASEPRI, r2
//    0x0800030e <+28>:    movw    r2, #0
// => 0x08000312 <+32>:    bkpt    0x0000
//    0x08000314 <+34>:    movt    r2, #8192       ; 0x2000
//    0x08000318 <+38>:    ldr     r3, [r2, #0]
//    0x0800031a <+40>:    subs    r0, r3, #1
//    0x0800031c <+42>:    cmp     r0, #8
//    0x0800031e <+44>:    ittt    ls
//    0x08000320 <+46>:    ldrls   r0, [r2, #4]
//    0x08000322 <+48>:    addls   r0, r3
//    0x08000324 <+50>:    strls   r0, [r2, #4]
//    0x08000326 <+52>:    bkpt    0x0000
//    0x08000328 <+54>:    msr     BASEPRI, r1
// ...
//
// We are now inside the critital sections of both X and Y and have access to both resources for the loop.
// (gdb) c
// Continuing.
//
// Program received signal SIGTRAP, Trace/breakpoint trap.
// cortex_m_rtfm::claim (ceiling=3, _nvic_prio_bits=4, data=<optimized out>, t=<optimized out>, f=...)
//     at /home/pln/klee/cortex-m-rtfm/src/lib.rs:181
//
// We are now releasing `Y`.
// (gdb) mon mrw 0xe0001004
// 29
//
// How long (in cycles) was the critical section on `Y`?
// ** your answer here **

// (gdb) disassemble
// ...
//    0x08000322 <+48>:    addls   r0, r3
//    0x08000324 <+50>:    strls   r0, [r2, #4]
// => 0x08000326 <+52>:    bkpt    0x0000
//    0x08000328 <+54>:    msr     BASEPRI, r1
// ...
//
// Let us continue
// (gdb) c
// Continuing.

// Program received signal SIGTRAP, Trace/breakpoint trap.
// cortex_m_rtfm::claim (ceiling=2, _nvic_prio_bits=4, data=<optimized out>, t=<optimized out>, f=...)
//     at /home/pln/klee/cortex-m-rtfm/src/lib.rs:181
// 181                             bkpt();
//
// (gdb) mon mrw 0xe0001004
// 30
//
// How long (in cycles) was the critical section on `X`?
// ** your answer here **
//
// (gdb) disassemble
// ...
//    0x08000326 <+52>:    bkpt    0x0000
//    0x08000328 <+54>:    msr     BASEPRI, r1
// => 0x0800032c <+58>:    bkpt    0x0000
//    0x0800032e <+60>:    msr     BASEPRI, r12
//    0x08000332 <+64>:    bx      lr
//
// At this point we can contiue
// (gdb) c
// Continuing.
// EXTI1 has now run to completion.
//
// What was the total excecution time for EXTI1?
// ** your answer here **
//
// What is the value of the resouce X?
// ** your answer here **
//
// Just by looking at the code, what would the worst case assignment of X be?
// I.e., the assingnment of `X' that gives the worst case execution time.
// ** your answer here **
//
// Measure that execution time. What did you get.
// ** your answer here **
//
// Your answer might blow your mind!!! How the heck is that even possible?
//
// At this point you should find that the worst case time stays the same ...
//
// Well, this one is not due to RTFM, its due to Rust and the cleverness of LLVM.
// What it does is that it figures out that we actually increase `Y`, by the value of `X`.
// So the loop is all gone, optimized out, and replaced by a simple addition.
//
// Now lets look at best case execution time.
// Figure out an assignment of `X`, that might actually reduce the execution time.
// ** your answer here **
//
// Redo the experiment with this value. How many clock cycles did you get?
// ** your answer here **
//
// Assignment 3.
// In assignment 2 we analysed EXTI1
// We found that for the examples, Rust RTFM was able to generate extremely efficient
// implementations, replacing branching with contdiditonal assignments and
// even remove comlete loops.
//
// Now lets have a look at using optimized LLVM and how it affects the number of tests.
// > xargo build --example resource --release --features klee_mode --target x86_64-unknown-linux-gnu
//
// and now start a docker for the optimized build
// > docker run --rm --user $(id -u):$(id -g) -v $PWD/target/x86_64-unknown-linux-gnu/release/examples:/mnt -w /mnt -it afoht/llvm-klee-4 /bin/bash
//
// and in the docker run
// > klee resource*.bc
//
// Look at the generated tests.
// Motivate for each of the 5 test cases, which one it matches of the hand generated tests from Assignment 2.
// ** your answers (5) here **
//
// Were the optimized tests sufficient to cover the execution time behavior.
// ** your answer here **
//
// Can you come up with a case where --release mode based analysis would miss cricital cases.
// ** your answer here, --- actually a research question, we will discuss in class --- **
//
// On a side note.
// Rust in --release mode makes the job for KLEE much easier. Look the number of instuctions carried out.
// In dev/debug you had som 10 000 instuctions executed for generating the test cases.
// In --relesase you had 34!!!!!
//
// This vastly improves on the performance of analysis, allowing larger applications to be scrutenized.
// The code we looked at here was by intention simple (to facilitate inspection).
// However, the code is NOT trivial, under the hood it utilizes advanced language features, such as
// closures and intence use of abstractions, such as Traits/genecics etc. It indeed covers a large
// degree of the Rust, for which it demonstrates that our approach to based program analysis
// actually works.
//
// For the future we intend the framework to cover the reading and writing to peripherals, and the
// sequenced access to resources. In class we will further discuss current limitations, and
// oportunities to improvements.
