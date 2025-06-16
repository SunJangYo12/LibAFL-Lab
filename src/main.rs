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
    fuzzer::StdFuzzer,
    generators::RandPrintablesGenerator,
};

use libafl_bolts::{
    rands::StdRand,
    AsSlice,
    nonzero,
};

use std::path::PathBuf;

fn main()
{
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.as_slice();

        if buf.len() > 0 && buf[0] == 'a' as u8 {
            if buf.len() > 1 && buf[1] == 'b' as u8 {
                if buf.len() > 2 && buf[2] == 'c' as u8 {
                    panic!("=)");
                }
            }
        }
        ExitKind::Ok
    };


    let mut state = StdState::new(
        StdRand::new(),
        InMemoryCorpus::<BytesInput>::new(),
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
        &mut (),
        &mut (),
    )
    .unwrap();


    let mon = SimpleMonitor::new(|s| println!("{s}"));
    let mut mgr = SimpleEventManager::new(mon);


    let scheduler = QueueScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, (), ());

    let mut executor = InProcessExecutor::new(&mut harness, (), &mut fuzzer, &mut state, &mut mgr).expect("Failed to create the Executor");


    let mut generator = RandPrintablesGenerator::new(nonzero!(32));

    state.generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8).expect("Failed to generate the initial corpus");
}
