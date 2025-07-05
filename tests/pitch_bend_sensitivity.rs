use mpe_state::{MPEState, Zone};

#[test]
fn member_channel_pitch_bend_sensitivity() {
    let mut mpe = MPEState::new();
    // config zones so channel 7 remains conventional
    mpe.config(&Zone::Lower, 6);
    mpe.config(&Zone::Upper, 7);
    // manager channel
    mpe.set_pitch_bend_sensitivity(0, 1);
    // conventional channel
    mpe.set_pitch_bend_sensitivity(7, 3);
    // member channel, the sensitivity should be applied
    // to all member channels of the zone
    mpe.set_pitch_bend_sensitivity(3, 12);
    // check manager channel
    assert_eq!(mpe.get_channel(0).unwrap().pitch_bend_sensitivity(), 1);
    // check conventional channel
    assert_eq!(mpe.get_channel(7).unwrap().pitch_bend_sensitivity(), 3);
    // check member channel
    assert_eq!(mpe.get_channel(3).unwrap().pitch_bend_sensitivity(), 12);
    // check other zone member channel
    assert_eq!(mpe.get_channel(10).unwrap().pitch_bend_sensitivity(), 48);
    // check other zone manager channel
    assert_eq!(mpe.get_channel(15).unwrap().pitch_bend_sensitivity(), 2);
}
