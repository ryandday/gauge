// @task(P1-T5) Implement session persistence: save/load to ~/.sherpa/sessions/<hash>.json
mod persistence;

#[allow(unused_imports)]
pub use persistence::{
    delete_session, load_session, save_session, session_exists, session_path, sessions_dir,
    SessionLoadResult,
};
