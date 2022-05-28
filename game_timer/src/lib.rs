use std::time::{Instant, Duration};
use std::fmt::{self,Display};
use std::cmp::min;

#[derive(Default, Debug)]
pub struct TimerState{
    started: Option<Instant>
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

impl TimerState{
    pub fn reset(&mut self) {
        self.started = None;
        self.inspection_end = None;
        self.ended = None;
    }

    pub fn game_id(&self) -> Option<String> {
        self.started.map(|i| format!("{:?}", i))
    }

    pub fn is_inspecting(&self) -> bool {
        self.started.is_some() && self.inspection_end.is_none()
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
        if self.is_inspecting(){
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
            self.inspection_end = None;
            self.ended = None;
            true
        }
        else {
            false
        }
    }

    pub fn solved(&mut self) -> bool {
        if self.ended.is_some() {
            false
        }
        else {
            self.ended = Some(Instant::now());
            true
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

    pub fn serialise(&self) -> (String, String, String){
        match self.started{
            None => {("X".to_string(), "X".to_string(), "X".to_string())}
            Some(start) => {
                match self.inspection_end {
                    None => {("0".to_string(), "X".to_string(), "X".to_string())}
                    Some(inspect) => {
                        let d_in = format!("{}", (inspect - start).as_millis());
                        match self.ended{
                            None => { ("0".to_string(),d_in,"X".to_string()) }
                            Some(end) => {
                                let d_tot = format!("{}", (end - start).as_millis());
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

    pub fn deserialise(start: Instant, durs: (String, String, String)) -> Result<Self, ()>{
        let (s, i, e) = durs;
        let add_start = |d|Some(start+d);
        Ok(TimerState{
            started: Self::str_to_opt_dir(s)?.and_then(add_start)
            ,inspection_end: Self::str_to_opt_dir(i)?.and_then(add_start)
            ,ended: Self::str_to_opt_dir(e)?.and_then(add_start)
        })
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
        assert!(!state.is_inspecting());
        assert!(!state.is_ended());
        // we can start the timer
        assert!(state.can_start());
        assert!(state.start());
        assert!(state.is_started());
        assert!(state.is_inspecting());
        assert!(!state.is_ended());
        // but we can't start it again yet
        assert!(!state.can_start());
        assert!(!state.start());
        assert!(state.is_started());
        assert!(state.is_inspecting());
        assert!(!state.is_ended());
        // after the first twist, inspection has ended
        assert!(state.is_inspecting());
        assert!(state.twist());
        assert!(state.is_started());
        assert!(!state.is_inspecting());
        assert!(!state.is_ended());
        // The next twist returns false to indicate it didn't end the inspection
        assert!(!state.twist());
        assert!(state.is_started());
        assert!(!state.is_inspecting());
        assert!(!state.is_ended());
        // Finally, a solve completes the timer
        assert!(state.solved());
        assert!(!state.solved()); // Can't solve the same scramble twice, needs reset between
        assert!(state.is_started());
        assert!(!state.is_inspecting());
        assert!(state.is_ended());
        let (start, iend, end) = state.serialise();
        assert!(&start != "X");
        assert!(&iend != "X");
        assert!(&end != "X");
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
