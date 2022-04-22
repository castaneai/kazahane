use binrw::binrw;

#[binrw]
#[brw(little, magic = b"KAZAHANE 1.0.0")]
struct KazahanePacket {
    packet_type: PacketType,
    payload_size: u16,
    #[br(count = payload_size)]
    payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
enum PacketType {
    HelloRequest = 0x01,
}

#[test]
fn parse_kazahane_packet() {
    use binrw::io::Cursor;
    use binrw::BinRead;

    let mut data = Cursor::new(b"KAZAHANE 1.0.0\x01\x05\x00hello");
    let p = KazahanePacket::read(&mut data).unwrap();
    assert_eq!(PacketType::HelloRequest, p.packet_type);
    assert_eq!(b"hello".to_vec(), p.payload);
}
