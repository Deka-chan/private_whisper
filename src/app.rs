#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    HotkeyPressed,
    TranscriptionDone,
    TranscriptionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    StartRecording,
    StopAndTranscribe,
}

/// Pure toggle state machine. No IO — the caller performs the returned Action.
pub fn transition(state: State, event: Event) -> (State, Action) {
    use Action::*;
    use Event::*;
    use State::*;
    match (state, event) {
        (Idle, HotkeyPressed) => (Recording, StartRecording),
        (Recording, HotkeyPressed) => (Transcribing, StopAndTranscribe),
        (Transcribing, TranscriptionDone) => (Idle, None),
        (Transcribing, TranscriptionFailed) => (Idle, None),
        // Everything else is a safe no-op that preserves the current state.
        (s, _) => (s, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_hotkey_starts_recording() {
        assert_eq!(
            transition(State::Idle, Event::HotkeyPressed),
            (State::Recording, Action::StartRecording)
        );
    }

    #[test]
    fn recording_hotkey_stops_and_transcribes() {
        assert_eq!(
            transition(State::Recording, Event::HotkeyPressed),
            (State::Transcribing, Action::StopAndTranscribe)
        );
    }

    #[test]
    fn transcribing_done_returns_to_idle() {
        assert_eq!(
            transition(State::Transcribing, Event::TranscriptionDone),
            (State::Idle, Action::None)
        );
    }

    #[test]
    fn transcribing_failed_returns_to_idle() {
        assert_eq!(
            transition(State::Transcribing, Event::TranscriptionFailed),
            (State::Idle, Action::None)
        );
    }

    #[test]
    fn hotkey_ignored_while_transcribing() {
        assert_eq!(
            transition(State::Transcribing, Event::HotkeyPressed),
            (State::Transcribing, Action::None)
        );
    }

    #[test]
    fn stray_completion_events_are_noops() {
        assert_eq!(
            transition(State::Idle, Event::TranscriptionDone),
            (State::Idle, Action::None)
        );
        assert_eq!(
            transition(State::Recording, Event::TranscriptionFailed),
            (State::Recording, Action::None)
        );
    }
}
