use config::Task;
use actions::*;
use errors::JobError;

use std::sync::{Arc, Mutex};

use scoped_pool::Pool;
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use futures::sync::oneshot;
use futures::Stream;


pub struct Queue {
    thread_pool: Pool,
    jobs_reader: UnboundedReceiver<Job>,
}

pub struct Job {
    pub image_id: u64,
    pub image_path: String,
    pub task: Task,
    pub response: Option<oneshot::Sender<()>>,
}

impl Queue {
    pub fn new(pool_size: usize, jobs_reader: UnboundedReceiver<Job>) -> Queue {
        Queue {
            thread_pool: Pool::new(pool_size),
            jobs_reader: jobs_reader,
        }
    }

    pub fn run_job(&self, job: &Job) -> Result<(), JobError> {
        println!("QS JO: {:?}", job.image_path);
        // let mut img = try!(image::open(job.image_path.clone()).map_err(|e| JobError::Image(e)));

        let mut imgd = try!(ImageData::new(job.image_path.clone(), job.image_id)
            .map_err(|e| JobError::Image(e)));
        println!("QS IMAGE_DATA OK");

        for action in job.task.actions.iter() {
            imgd = try!(action.run(&mut imgd).map_err(|e| JobError::Action(e)));
        }

        Ok(())
    }

    pub fn run(&self) {
        self.thread_pool
            .scoped(|scope| for job in self.jobs_reader.wait() {
                        match job {
                            Ok(job) => {
                    println!("JOB START");
                    scope.execute(move || {
                        match self.run_job(&job) {
                            Err(e) => {
                                println!("Error processing job: {:?}", e);
                            }
                            Ok(j) => j,
                        };

                        if let Some(ref response) = job.response {
                            response.complete(());
                        }
                    });
                }
                            Err(e) => println!("Error receiving job: {:?}", e),
                        }
                    });
    }
}
