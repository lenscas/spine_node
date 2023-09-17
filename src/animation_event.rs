use rusty_spine::{Event, TrackEntry};

#[derive(Debug)]
pub enum AnimationEvent {
    Start {
        /// The track this event originated from.
        track_entry: TrackEntry,
    },
    Interrupt {
        /// The track this event originated from.
        track_entry: TrackEntry,
    },
    End {
        /// The track this event originated from.
        track_entry: TrackEntry,
    },
    Complete {
        /// The track this event originated from.
        track_entry: TrackEntry,
    },
    Dispose {
        /// The track this event originated from.
        track_entry: TrackEntry,
    },
    Event {
        /// The track this event originated from.
        track_entry: TrackEntry,
        /// The name of the event, which is unique across all events in the skeleton.
        name: String,
        /// The animation time this event was keyed.
        time: f32,
        /// The event's int value.
        int: i32,
        /// The event's float value.
        float: f32,
        /// The event's string value or an empty string.
        string: String,
        /// The event's audio path or an empty string.
        audio_path: String,
        /// The event's audio volume.
        volume: f32,
        /// The event's audio balance.
        balance: f32,
        /// The raw event data.
        event: Event,
    },
}

impl<'a> From<rusty_spine::AnimationEvent<'a>> for AnimationEvent {
    fn from(value: rusty_spine::AnimationEvent) -> Self {
        match value {
            rusty_spine::AnimationEvent::Start { track_entry } => {
                AnimationEvent::Start { track_entry }
            }
            rusty_spine::AnimationEvent::Interrupt { track_entry } => {
                AnimationEvent::Interrupt { track_entry }
            }
            rusty_spine::AnimationEvent::End { track_entry } => AnimationEvent::End { track_entry },
            rusty_spine::AnimationEvent::Complete { track_entry } => {
                AnimationEvent::Complete { track_entry }
            }
            rusty_spine::AnimationEvent::Dispose { track_entry } => {
                AnimationEvent::Dispose { track_entry }
            }
            rusty_spine::AnimationEvent::Event {
                track_entry,
                name,
                time,
                int,
                float,
                string,
                audio_path,
                volume,
                balance,
                event,
            } => AnimationEvent::Event {
                track_entry,
                name: name.to_string(),
                time,
                int,
                float,
                string: string.to_string(),
                audio_path: audio_path.to_string(),
                volume,
                balance,
                event,
            },
        }
    }
}
