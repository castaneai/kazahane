use binrw::binrw;

#[derive(Debug)]
#[binrw]
#[brw(little, magic = b"KAZAHANE 1.0.0")]
pub struct KazahanePacket {
    pub packet_type: PacketType,
    pub payload_size: u16,
    #[br(count = payload_size)]
    pub payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
pub enum PacketType {
    HelloRequest = 0x01,
}

#[test]
fn read_packet() {
    use binrw::io::Cursor;
    use binrw::BinRead;

    let mut data = Cursor::new(b"KAZAHANE 1.0.0\x01\x05\x00hello");
    let p = KazahanePacket::read(&mut data).unwrap();
    assert_eq!(PacketType::HelloRequest, p.packet_type);
    assert_eq!(b"hello".to_vec(), p.payload);
}

#[test]
fn write_packet() {
    use binrw::io::Cursor;
    use binrw::BinWrite;

    let mut writer = Cursor::new(Vec::new());
    let packet = KazahanePacket{
        packet_type: PacketType::HelloRequest,
        payload_size: 5,
        payload: b"world".to_vec(),
    };
    packet.write_to(&mut writer).unwrap();
    assert_eq!(&writer.into_inner()[..], b"KAZAHANE 1.0.0\x01\x05\x00world");
}