use core::time::Duration;
use std::{env, net::SocketAddr, path::PathBuf};

use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    events::{launcher::Launcher, EventConfig},
    executors::{inprocess::InProcessExecutor, ExitKind},
    feedback_or, feedback_or_fast,
    feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback, TimeoutFeedback},
    fuzzer::{Fuzzer, StdFuzzer},
    inputs::{BytesInput, HasTargetBytes},
    monitors::{MultiMonitor, OnDiskTOMLMonitor},
    mutators::{
        scheduled::{havoc_mutations, StdScheduledMutator},
    },
    observers::{CanTrack, HitcountsMapObserver, TimeObserver},
    schedulers::{IndexesLenTimeMinimizerScheduler, QueueScheduler},
    stages::mutational::StdMutationalStage,
    state::{HasCorpus, StdState},
    Error,
};
use libafl_bolts::{
    core_affinity::Cores,
    rands::StdRand,
    shmem::{ShMemProvider, StdShMemProvider},
    tuples::{tuple_list},
    AsSlice,
};
use libafl_targets::{libfuzzer_initialize, libfuzzer_test_one_input, std_edges_map_observer};


/// The main fn, `no_mangle` as it is a C symbol
#[no_mangle]
pub extern "C" fn libafl_main() {

    println!(
        "Workdir: {:?}",
        env::current_dir().unwrap().to_string_lossy().to_string()
    );

    let mut run_client = |state: Option<_>, mut restarting_mgr, _core_id| {
        // Create an observation channel using the coverage map
        let edges_observer =
            HitcountsMapObserver::new(unsafe { std_edges_map_observer("edges") }).track_indices();

        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        // Feedback to rate the interestingness of an input
        // This one is composed of two feedbacks in OR
        let mut feedback = feedback_or!(
            // New maximization map feedback linked to the edges observer
            MaxMapFeedback::new(&edges_observer),
            // Time feedback
            TimeFeedback::new(&time_observer)
        );

        // A feedback to choose if an input is a solution or not
        let mut objective = feedback_or_fast!(CrashFeedback::new(), TimeoutFeedback::new());

        // If not restarting, create a State from scratch
        let mut state = state.unwrap_or_else(|| {
            StdState::new(
                // RNG
                StdRand::new(),
                // Corpus that will be evolved, we keep it in memory for performance
                InMemoryCorpus::new(),
                // Corpus in which we store solutions (crashes in this example),
                // on disk so the user can get them after stopping the fuzzer
                OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
                // States of the feedbacks.
                // The feedbacks can report the data that should persist in the State.
                &mut feedback,
                // Same for objective feedbacks
                &mut objective,
            )
            .unwrap()
        });

        println!("We're a client, let's fuzz :)");

        // Setup a basic mutator with a mutational stage
        let mutator = StdScheduledMutator::new(havoc_mutations());
        let mut stages = tuple_list!(StdMutationalStage::new(mutator));

        // A minimization+queue policy to get testcasess from the corpus
        let scheduler =
            IndexesLenTimeMinimizerScheduler::new(&edges_observer, QueueScheduler::new());

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        // The wrapped harness function, calling out to the LLVM-style harness
        let mut harness = |input: &BytesInput| {
            let target = input.target_bytes();
            let buf = target.as_slice();
            libfuzzer_test_one_input(buf);
            ExitKind::Ok
        };

        let mut executor = InProcessExecutor::with_timeout(
            &mut harness,
            tuple_list!(edges_observer, time_observer),
            &mut fuzzer,
            &mut state,
            &mut restarting_mgr,
            Duration::new(10, 0),
        )?;

        // The actual target run starts here.
        // Call LLVMFUzzerInitialize() if present.
        let args: Vec<String> = env::args().collect();
        if libfuzzer_initialize(&args) == -1 {
            println!("Warning: LLVMFuzzerInitialize failed with -1");
        }


        let corpus_dirs = vec![PathBuf::from("./corpus")];
        // In case the corpus is empty (on first run), reset
        if state.must_load_initial_inputs() {
            state
                .load_initial_inputs(&mut fuzzer, &mut executor, &mut restarting_mgr, &corpus_dirs)
                .unwrap_or_else(|_| panic!("Failed to load initial corpus at {:?}", &corpus_dirs));
            println!("We imported {} inputs from disk.", state.corpus().count());
        }
        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut restarting_mgr)
    };




    let shmem_provider = StdShMemProvider::new().expect("Failed to init shared memory");

    let monitor = OnDiskTOMLMonitor::new(
        "./fuzzer_stats.toml",
        MultiMonitor::new(|s| println!("{s}")),
    );

    let broker_port = 1337;
    let cores = Cores::all().unwrap();

    let addr: SocketAddr = "192.168.0.101:1337".parse().expect("Invalid ip");

    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .configuration(EventConfig::from_name("default"))
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .broker_port(broker_port)
        .remote_broker_addr(None) //server atau broker
        //.remote_broker_addr(Some(addr)) //client
        //.stdout_file(Some("/dev/null"))
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => println!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {err:?}"),
    }
}