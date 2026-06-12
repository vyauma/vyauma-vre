use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

/// A message passed between actors.
#[derive(Debug, Clone)]
pub struct Message {
    pub sender_id: String,
    pub payload: String, // Simplified payload for now
}

/// Reference to an Actor that allows sending messages to it.
#[derive(Clone)]
pub struct ActorRef {
    pub id: String,
    sender: Sender<Message>,
}

impl ActorRef {
    pub fn send(&self, msg: Message) {
        let _ = self.sender.send(msg);
    }
}

/// The Actor System manages the lifecycle and message routing for Actors.
pub struct ActorSystem {
    actors: Arc<Mutex<HashMap<String, ActorRef>>>,
}

impl ActorSystem {
    pub fn new() -> Self {
        ActorSystem {
            actors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a new actor with a given closure to handle messages.
    pub fn spawn<F>(&self, id: &str, mut handler: F) -> ActorRef
    where
        F: FnMut(Message) + Send + 'static,
    {
        let (tx, rx): (Sender<Message>, Receiver<Message>) = channel();
        let actor_ref = ActorRef {
            id: id.to_string(),
            sender: tx,
        };

        let mut actors = self.actors.lock().unwrap();
        actors.insert(id.to_string(), actor_ref.clone());

        let actor_id = id.to_string();
        thread::spawn(move || {
            println!("Actor {} started.", actor_id);
            for msg in rx {
                handler(msg);
            }
            println!("Actor {} stopped.", actor_id);
        });

        actor_ref
    }

    /// Retrieve an actor by ID
    pub fn get_actor(&self, id: &str) -> Option<ActorRef> {
        let actors = self.actors.lock().unwrap();
        actors.get(id).cloned()
    }
}
