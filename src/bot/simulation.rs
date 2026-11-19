use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use super::search::{ACTIONS, SearchContext, SearchResult, SearchWorker, choose_result};
use crate::resources::Resources;

struct Job {
    number: usize,
    context: Arc<SearchContext>,
}

struct JobResult {
    number: usize,
    best: Option<SearchResult>,
}

pub(super) struct SimulationControl {
    jobs: Option<mpsc::Sender<Job>>,
    results: mpsc::Receiver<JobResult>,
    workers: Vec<JoinHandle<()>>,
}

impl SimulationControl {
    pub fn new(resources: Arc<Resources>) -> Self {
        let (job_sender, job_receiver) = mpsc::channel::<Job>();
        let (result_sender, result_receiver) = mpsc::channel();
        // A standard channel has one receiver. Arc shares its ownership,
        // and the mutex lets one worker at a time take the next job.
        let job_receiver = Arc::new(Mutex::new(job_receiver));
        let mut workers = Vec::new();

        // Exactly four workers are created once, when the bot is constructed.
        for _ in 0..4 {
            let jobs = Arc::clone(&job_receiver);
            let results = result_sender.clone();
            let resources = Arc::clone(&resources);
            workers.push(thread::spawn(move || {
                let mut search = SearchWorker::new();
                loop {
                    // Release the queue lock before searching. Other workers
                    // can receive their jobs while this one is computing.
                    let job = {
                        let receiver = jobs.lock().unwrap();
                        receiver.recv()
                    };
                    let job = match job {
                        Ok(job) => job,
                        Err(_) => break,
                    };
                    let best = search.search(&job.context, &resources.config, ACTIONS[job.number]);
                    // Send one result even if no valid landing was found.
                    results
                        .send(JobResult {
                            number: job.number,
                            best,
                        })
                        .unwrap();
                }
            }));
        }
        Self {
            jobs: Some(job_sender),
            results: result_receiver,
            workers,
        }
    }

    pub fn search(&self, context: Arc<SearchContext>) -> Option<SearchResult> {
        let mut results = Vec::new();
        for number in 0..ACTIONS.len() {
            results.push(None);
            self.jobs
                .as_ref()
                .unwrap()
                .send(Job {
                    number,
                    context: Arc::clone(&context),
                })
                .unwrap();
        }
        // Finish this search completely before the bot advances a frame.
        // There are no unfinished jobs or old results mixed into the next call.
        for _ in 0..ACTIONS.len() {
            let result = self.results.recv().unwrap();
            results[result.number] = result.best;
        }
        choose_result(results)
    }

    #[cfg(test)]
    pub fn worker_ids(&self) -> Vec<thread::ThreadId> {
        let mut ids = Vec::new();
        for worker in &self.workers {
            ids.push(worker.thread().id());
        }
        ids
    }
}

impl Drop for SimulationControl {
    fn drop(&mut self) {
        // Closing the sole sender wakes idle receivers. Joining before closing
        // it would leave the workers waiting forever for another job.
        drop(self.jobs.take());
        while let Some(worker) = self.workers.pop() {
            worker.join().unwrap();
        }
    }
}
