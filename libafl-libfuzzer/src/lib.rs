use std::path::PathBuf;
use std::time::Duration;

use libafl::corpus::{Corpus, InMemoryCorpus, OnDiskCorpus};
use libafl::events::{setup_restarting_mgr_std, EventConfig, EventRestarter};
use libafl::executors::{ExitKind, InProcessExecutor};
use libafl::feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback};
use libafl::inputs::{BytesInput, HasTargetBytes};
use libafl::monitors::MultiMonitor;
use libafl::mutators::{havoc_mutations, StdScheduledMutator};
use libafl::observers::{CanTrack, HitcountsMapObserver, TimeObserver};
use libafl::schedulers::{IndexesLenTimeMinimizerScheduler, QueueScheduler};
use libafl::stages::StdMutationalStage;
use libafl::state::{HasCorpus, StdState};
use libafl::{feedback_and_fast, feedback_or, Error, Fuzzer, StdFuzzer};
use libafl_bolts::rands::StdRand;
use libafl_bolts::tuples::tuple_list;
use libafl_bolts::{current_nanos, AsSlice};
use libafl_targets::{libfuzzer_test_one_input, std_edges_map_observer};

#[no_mangle]
fn libafl_main() -> Result<(), Error> {

    // Component: Corpus
    let corpus_dirs = vec![PathBuf::from("./seeds")];
    let input_corpus = InMemoryCorpus::<BytesInput>::new();
    let solutions_corpus = OnDiskCorpus::new(PathBuf::from("./crash")).unwrap();


    // Component: Observer
    let edges_observer = HitcountsMapObserver::new(unsafe { //mengamati coverage dengan memanfaatkan sancov instrumnen
        std_edges_map_observer("edges")
    }).track_indices();
    let time_observer = TimeObserver::new("time"); //mengukur wakti eksekusi input


    // Component: Feedback
    let mut feedback = feedback_or!(  //menentukan apakah sebuah input bernilai cukup menarik untuk disimpan
        MaxMapFeedback::new(&edges_observer),
        TimeFeedback::new(&time_observer)
    );

    // menentukan apakah input dianggap berhasil (misalnya crash) dan ditandai sebagai solusi.
    let mut objective = feedback_and_fast!(CrashFeedback::new(), MaxMapFeedback::new(&edges_observer));



    // Component: Monitor
    let monitor = MultiMonitor::new(|s| { //mencetak output ke stdout
        println!("{}", s);
    });

    // Component: EventManager
    /* mgr = EventManager mengatur komunikasi antara fuzzer dan thread / worker lain
     * setup_restarting_mgr_std = memungkinkan restart otomatis setelah crash
    */
    let (state, mut mgr) = match setup_restarting_mgr_std(monitor, 1337, EventConfig::AlwaysUnique)
    {
        Ok(res) => res,
        Err(err) => match err {
            Error::ShuttingDown => {
                return Ok(());
            }
            _ => {
                panic!("Failed to setup the restarting manager: {}", err);
            }
        },
    };


    // Component: State
    let mut state = state.unwrap_or_else(|| {
        StdState::new(
            StdRand::with_seed(current_nanos()),
            input_corpus,
            solutions_corpus,
            &mut feedback,
            &mut objective,
        )
        .unwrap()
    });


    // Component: Scheduler
    let scheduler = IndexesLenTimeMinimizerScheduler::new(&edges_observer, QueueScheduler::new());
    // menentukan urutan input mana dari corpus yang akan difuzz berikutnya
    // strategi: memproitaskan input dengan index tertentu, pendek atau cepat dijalankan


    // Component: Fuzzer
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective); //object fuzzer utama yang menjalankan loop. menggunakan scheduler, feedback dan goal(objective/crash)

    // Component: harness
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes(); //fungsi target yang dipanggil fuzzer
        let buffer = target.as_slice();
        unsafe { libfuzzer_test_one_input(buffer) }; //input akan dipanggil ke fungsi C libfuzzzer_test_one_input.
        ExitKind::Ok                                 //fungsi ini berasal dari c/c++ target dan di link via libafl_targets
    };


    // Component: Executor
    let mut in_proc_executor = InProcessExecutor::with_timeout( //menjalankan harness dalam proses(singgle thread), lebih cepat.
        &mut harness,
        tuple_list!(edges_observer, time_observer),
        &mut fuzzer,
        &mut state,
        &mut mgr,
        Duration::from_millis(5000), //timeout diset agar infinite loop/input lambat bisa dihentikan
    )
    .unwrap();

    if state.corpus().count() < 1 { //load corpus awal, jika corpus masih kosong isi dari file input pada ./corpus
        state
            .load_initial_inputs(&mut fuzzer, &mut in_proc_executor, &mut mgr, &corpus_dirs)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to load initial corpus at {:?}: {:?}",
                    &corpus_dirs, err
                )
            });
        println!("We imported {} inputs from disk.", state.corpus().count());
    }

    // Component: Mutator
    let mutator = StdScheduledMutator::new(havoc_mutations()); //strategi mutasi acak klasik (insert,delete,bitflip, dsb)


    // Component: Stage
    let mut stages = tuple_list!(StdMutationalStage::new(mutator)); //yang akan menjalankan mutasi

    fuzzer.fuzz_loop_for( //menjalankan loop fuzzing selama 1000 iterasi, bisa diganti fuzz_loop() untuk tanpa batas
            &mut stages,
            &mut in_proc_executor,
            &mut state,
            &mut mgr,
            1000,
        )
        .unwrap();
    mgr.on_restart(&mut state).unwrap();

    Ok(())
}

