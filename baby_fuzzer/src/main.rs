/*
* Source: https://aflplus.plus/libafl-book/baby_fuzzer
* urutan tutorial import alias use libafl...
*/

extern crate libafl;
extern crate libafl_bolts;

use libafl::{
    executors::{ ExitKind, inprocess::InProcessExecutor },
    inputs::{ BytesInput, HasTargetBytes },
    state::StdState,
    corpus::{ InMemoryCorpus, OnDiskCorpus },
    monitors::SimpleMonitor,
    events::SimpleEventManager,
    schedulers::QueueScheduler,
    fuzzer::{ Fuzzer, StdFuzzer },
    generators::RandPrintablesGenerator,
    observers::StdMapObserver,
    feedbacks::{ CrashFeedback, MaxMapFeedback },
    mutators::{ havoc_mutations::havoc_mutations, scheduled::StdScheduledMutator },
    stages::mutational::StdMutationalStage,
};

use libafl_bolts::{
    rands::StdRand,
    AsSlice,
    nonzero,
    tuples::tuple_list,
};

use std::{ path::PathBuf, ptr::write };


// Coverage map with explicit assignments due to the lack of instrumentation
static mut SIGNALS: [u8; 16] = [0; 16];
#[allow(static_mut_refs)]
static mut SIGNALS_PTR: *mut u8 = unsafe { SIGNALS.as_mut_ptr() };

fn signals_set(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}


fn main()
{
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.as_slice();

        signals_set(0);

        if buf.len() > 0 && buf[0] == 'a' as u8 
        {
            signals_set(1);
            if buf.len() > 1 && buf[1] == 'b' as u8 
            {
                signals_set(2);
                if buf.len() > 2 && buf[2] == 'c' as u8 
                {
                    panic!("=)");
                }
            }
        }
        ExitKind::Ok
    };

    #[allow(static_mut_refs)]
    let observer = unsafe { StdMapObserver::from_mut_ptr("signals", SIGNALS_PTR, SIGNALS.len()) };

    let mut feedback = MaxMapFeedback::new(&observer);
    let mut objective = CrashFeedback::new();


    let mut state = StdState::new(
        StdRand::new(),
        InMemoryCorpus::<BytesInput>::new(),
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
        &mut feedback,
        &mut objective,
    )
    .unwrap();


    let mon = SimpleMonitor::new(|s| println!("{s}"));
    let mut mgr = SimpleEventManager::new(mon);


    let scheduler = QueueScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let mut executor = InProcessExecutor::new(
        &mut harness, 
        tuple_list!(observer),
        &mut fuzzer,
        &mut state,
        &mut mgr)
    .expect("Failed to create the Executor");


    let mut generator = RandPrintablesGenerator::new(nonzero!(32));

    state.generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8).expect("Failed to generate the initial corpus");


    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mut stages = tuple_list!(StdMutationalStage::new(mutator));

    fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr).expect("Error in the fuzzing loop");

}
