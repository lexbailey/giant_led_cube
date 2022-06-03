use std::time::{Instant, Duration};
use std::fmt::{self,Display};
use std::cmp::min;
use uuid::Uuid;

#[derive(Default, Debug)]
pub struct TimerState{
    game_id: Option<Uuid>
    ,started: Option<Instant>
    ,inspection_end: Option<Instant>
    ,ended: Option<Instant>
}

impl Display for TimerState{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.started{
            Some(_s) => "t0"
            ,None => "?"
        };
        let i = match (self.started, self.inspection_end) {
            (Some(start), Some(end)) => {format!("{:#?}", end-start)}
            ,_=>{"?".to_string()}
        };
        let e = match (self.started, self.ended) {
            (Some(start), Some(end)) => {format!("{:#?}", end-start)}
            ,_=>{"?".to_string()}
        };
        write!(f, "(s:{}, i:{}, e:{})", s,i,e)
    }
}

fn round_ms(d: Duration) -> Duration {
    Duration::from_millis(d.as_millis().try_into().unwrap_or(0))
}

impl TimerState{
    pub fn reset(&mut self) {
        self.game_id = None;
        self.started = None;
        self.inspection_end = None;
        self.ended = None;
    }

    pub fn game_id(&self) -> Option<Uuid> {
        self.game_id
    }

    pub fn is_inspecting(&self, t: Option<Instant>) -> bool {
        match t {
            None => {self.started.is_some() && self.inspection_end.is_none()}
            Some(t) => {self.started.is_some() && self.inspection_end.is_none() && t - self.started.unwrap() < Duration::new(15,0)}
        }
    }

    pub fn is_started(&self) -> bool {
        self.started.is_some()
    }

    pub fn is_ended(&self) -> bool {
        self.ended.is_some()
    }

    pub fn can_start(&self) -> bool {
        self.is_ended() || !self.is_started()
    }

    pub fn recorded_time(&self) -> Option<Duration>{
        match (self.started, self.inspection_end, self.ended) {
            (Some(start), Some(inspect_end), Some(end)) => {
                const FIFTEEN: Duration = Duration::from_secs(15);
                Some((end - start)- min(inspect_end - start, FIFTEEN))
            }
            ,_=>{
                None
            }
        }
    }

    pub fn twist(&mut self) -> bool {
        if self.is_inspecting(None){
            self.inspection_end = Some(Instant::now());
            true
        }
        else {
            false
        }
    }

    pub fn start(&mut self) -> bool {
        if self.can_start() {
            self.started = Some(Instant::now());
            self.game_id = Some(Uuid::new_v4());
            self.inspection_end = None;
            self.ended = None;
            true
        }
        else {
            false
        }
    }

    pub fn solved(&mut self) -> bool {
        if self.started.is_none() {
            false
        }
        else{
            if self.ended.is_some() {
                false
            }
            else {
                self.ended = Some(Instant::now());
                true
            }
        }
    }

    pub fn duration_so_far(&self) -> Duration {
        if !self.is_started(){
            Duration::new(0,0)
        }
        else{
            Instant::now() - self.started.unwrap()
        }
    }

    pub fn inspection_so_far(&self, at: Option<Instant>) -> Duration {
        if !self.is_started(){
            Duration::new(0,0)
        }
        else{
            if self.is_inspecting(at){
                let t = Instant::now() - self.started.unwrap();
                if t > Duration::new(15,0){
                    Duration::new(15,0)
                }
                else{
                    round_ms(t)
                }
            }
            else{
                round_ms(self.inspection_end.unwrap() - self.started.unwrap())
            }
        }
    }

    pub fn effective_inspection_end(&self) -> Option<Instant>{
        self.inspection_end.and_then(|t|{
            let s = self.started.unwrap();
            if t-s > Duration::new(15,0) {
                Some(s + Duration::new(15,0))
            }
            else{
                Some(t)
            }
        })
    }

    pub fn solve_so_far(&self) -> Duration {
        let at = Instant::now();
        if self.is_ended() {
            round_ms(self.ended.unwrap() - self.effective_inspection_end().unwrap())
        }
        else if !self.is_started(){
            Duration::new(0,0)
        }
        else{
            if self.is_inspecting(Some(at)){
                Duration::new(0,0)
            }
            else{
                match self.effective_inspection_end() {
                    Some(e) => {Instant::now() - e}
                    ,None => {
                        let start = self.started.unwrap();
                        round_ms(at - start - Duration::new(15,0))
                    }
                }
            }
        }
    }

    pub fn serialise(&self) -> (String, String, String){
        match self.started{
            None => {("X".to_string(), "X".to_string(), "X".to_string())}
            Some(start) => {
                match self.inspection_end {
                    None => {("0".to_string(), "X".to_string(), "X".to_string())}
                    Some(inspect) => {
                        let d_in = format!("{}", round_ms(inspect - start).as_millis());
                        match self.ended{
                            None => { ("0".to_string(),d_in,"X".to_string()) }
                            Some(end) => {
                                let d_tot = format!("{}", round_ms(end - start).as_millis());
                                ("0".to_string(), d_in, d_tot)
                             }
                        }
                    }
                }
            }
        }
    }

    fn str_to_opt_dir(s: String) -> Result<Option<Duration>, ()>{
        if s == "X"{
            Ok(None)
        }
        else{
            let n: u64 = match s.parse() { Err(_) => return Err(()), Ok(a) => a };
            Ok(Some(Duration::from_millis(n)))
        }
    }

    fn deserialise_raw(start: Instant, durs: (Option<Duration>, Option<Duration>, Option<Duration>)) -> Result<Self, ()>{
        let (s, i, e) = durs;
        let add_start = |d|Some(start+d);
        Ok(TimerState{
            game_id: None
            ,started: s.and_then(add_start)
            ,inspection_end: i.and_then(add_start)
            ,ended: e.and_then(add_start)
        })
    }

    pub fn deserialise(start: Instant, durs: (String, String, String)) -> Result<Self, ()>{
        let (s, i, e) = durs;
        Self::deserialise_raw(start, (
            Self::str_to_opt_dir(s)?
            ,Self::str_to_opt_dir(i)?
            ,Self::str_to_opt_dir(e)?
        ))
    }

    pub fn deserialise_now_ish(durs: (String, String, String)) -> Result<Self, ()>{
        let t = Instant::now();
        let (s, i, e) = durs;
        let s = Self::str_to_opt_dir(s)?;
        let i = Self::str_to_opt_dir(i)?;
        let e = Self::str_to_opt_dir(e)?;
        let mut max = Duration::new(0,0);
        if let Some(t) = s {if t > max {max = t;}}
        if let Some(t) = i {if t > max {max = t;}}
        if let Some(t) = e {if t > max {max = t;}}
        Self::deserialise_raw(t - max, (s,i,e))
    }
}


#[cfg(test)]
mod tests {
    use crate::TimerState;
    use std::time::{Instant};
    #[test]
    fn basic_tests() {
        let mut state = TimerState::default();
        let (start, iend, end) = state.serialise();
        assert!(&start == "X");
        assert!(&iend == "X");
        assert!(&end == "X");
        assert!(!state.is_started());
        assert!(!state.is_inspecting(None));
        assert!(!state.is_ended());
        assert!(state.game_id().is_none());
        // we can start the timer
        assert!(state.can_start());
        assert!(state.start());
        assert!(state.is_started());
        assert!(state.is_inspecting(None));
        assert!(!state.is_ended());
        let first_game_id = state.game_id().expect("game id to be defined");
        // but we can't start it again yet
        assert!(!state.can_start());
        assert!(!state.start());
        assert!(state.is_started());
        assert!(state.is_inspecting(None));
        assert!(!state.is_ended());
        assert_eq!(state.game_id().unwrap(), first_game_id);
        // after the first twist, inspection has ended
        assert!(state.is_inspecting(None));
        assert!(state.twist());
        assert!(state.is_started());
        assert!(!state.is_inspecting(None));
        assert!(!state.is_ended());
        assert_eq!(state.game_id().unwrap(), first_game_id);
        // The next twist returns false to indicate it didn't end the inspection
        assert!(!state.twist());
        assert!(state.is_started());
        assert!(!state.is_inspecting(None));
        assert!(!state.is_ended());
        assert_eq!(state.game_id().unwrap(), first_game_id);
        // Finally, a solve completes the timer
        assert!(state.solved());
        assert!(!state.solved()); // Can't solve the same scramble twice, needs reset between
        assert!(state.is_started());
        assert!(!state.is_inspecting(None));
        assert!(state.is_ended());
        assert_eq!(state.game_id().unwrap(), first_game_id);
        let (start, iend, end) = state.serialise();
        assert!(&start != "X");
        assert!(&iend != "X");
        assert!(&end != "X");

        // Resetting results in no game id
        state.reset();
        assert!(state.game_id().is_none());
        // Starting a new game results in a new game id
        assert!(state.can_start());
        assert!(state.start());
        assert!(state.is_started());
        assert!(state.is_inspecting(None));
        assert!(!state.is_ended());
        assert_ne!(state.game_id().unwrap(), first_game_id);
    }

    #[test]
    fn ser_deser() {
        let n =  Instant::now();
        let blank_state = TimerState::deserialise(n, ("X".to_string(), "X".to_string(), "X".to_string())).unwrap();
        let started_state = TimerState::deserialise(n, ("0".to_string(), "X".to_string(), "X".to_string())).unwrap();
        let ended_state = TimerState::deserialise(n, ("0".to_string(), "15000".to_string(), "60000".to_string())).unwrap();
        assert_eq!(blank_state.serialise(), TimerState::default().serialise());
        assert_eq!(started_state.serialise(), ("0".to_string(), "X".to_string(), "X".to_string()));
        assert_eq!(ended_state.serialise(), ("0".to_string(), "15000".to_string(), "60000".to_string()));
    }
}
