use mpe_state::{Channel, MPEState, Zone};

#[test]
fn zone_overlap() {
	let mut state = MPEState::new();
	// [M L L L L L L L L L L L L L L L]
	// [1 --------------15-------------]
	state.config(Zone::Lower, 15);
	// [M L L L L L L L L L L U U U U M]
	// [1 --------10---------|---4--- 1]
	state.config(Zone::Upper, 4);
	assert_eq!(state.zone_member_channels(Zone::Lower).unwrap().len(), 10);
}

#[test]
fn zone_override() {
	let mut state = MPEState::new();
	state.config(Zone::Lower, 10);
	state.config(Zone::Upper, 4);
	// 14 member channels (15 zone channels) leave 1 unused channel
	// which should turn to conventional
	// because it can't have member channels
	// [M L L L L L L L L L L L L L L C]
	//                                ^
	//                                can't remain manager
	//                                no room for member channels
	state.config(Zone::Lower, 14);
	assert!(matches!(state.channels[Zone::Upper.manager_channel()], Channel::Conventional { .. }))
}

#[test]
fn mpe_deactivation() {
	let mut state = MPEState::new();
	state.config(Zone::Lower, 10);
	state.config(Zone::Upper, 4);
	assert_eq!(state.active(), true);
	state.config(Zone::Lower, 0);
	state.config(Zone::Upper, 0);
	assert_eq!(state.active(), false);
}
