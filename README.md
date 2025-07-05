# mpe-state

State management for MIDI Polyphonic Expression (MPE).

MIDI senders and receivers that implement MPE need to track the state shaped by past MIDI messages. This includes the MPE zone configuration and a number of parameters for each MIDI channel, including:

- pitch bend
- pitch bend sensitivity
- channel pressure
- timbre control (Control Change \#74)
- active notes and
- recently released notes.

## Goals

- Provide a comprehensive MIDI channel state, capable of supporting custom note allocation logic.
- Provide helper functions that implement mandatory and recommended MPE configuration practices.
- Use the terminology of the [MPE specification](https://midi.org/mpe-midi-polyphonic-expression).
- Remain agnostic to the MIDI message implementation.
