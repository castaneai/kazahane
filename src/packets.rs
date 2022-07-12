use anyhow::Context;
use binrw::{binrw, BinWrite};
use std::io::Cursor;

#[derive(Debug)]
#[binrw]
#[brw(little, magic = b"KAZAHANE 1.0.0")]
pub struct KazahanePacket {
    pub packet_type: PacketType,
    pub payload_size: u16,
    #[br(count = payload_size)]
    pub payload: Vec<u8>,
}

impl KazahanePacket {
    pub fn new<T: BinWrite>(packet_type: PacketType, bw: T) -> crate::Result<Self>
    where
        T::Args: Default,
    {
        let mut writer = Cursor::new(Vec::new());
        bw.write_to(&mut writer)
            .context("failed to write to kazahane packet")?;
        let vec = writer.into_inner();
        Ok(Self {
            packet_type,
            payload_size: vec.len() as u16,
            payload: vec,
        })
    }

    pub fn parse_payload<T>(&self) -> crate::Result<T>
    where
        T: binrw::BinRead,
        T::Args: Default,
    {
        let mut cursor = Cursor::new(&self.payload);
        T::read(&mut cursor).context("failed to parse")
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
pub enum PacketType {
    HelloRequest = 0x01,
    HelloResponse = 0x02,
    JoinRoomRequest = 0x03,
    JoinRoomResponse = 0x04,
}

#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct HelloRequestPacket {}

#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct JoinRoomRequestPacket {
    pub room_id: [u8; 16],
}

#[derive(Debug)]
#[binrw]
#[brw(little)]
pub struct HelloResponsePacket {
    pub status_code: HelloResponseStatusCode,
    pub message_size: u16,
    #[br(count = message_size)]
    pub message: Vec<u8>,
}

impl HelloResponsePacket {
    pub fn new(status_code: HelloResponseStatusCode, message: impl Into<Vec<u8>>) -> Self {
        let vec = message.into();
        Self {
            status_code,
            message_size: vec.len() as u16,
            message: vec,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
pub enum HelloResponseStatusCode {
    Unknown = 0x00,
    OK = 0x01,
    Denied = 0x02,
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
    let packet = KazahanePacket {
        packet_type: PacketType::HelloRequest,
        payload_size: 5,
        payload: b"world".to_vec(),
    };
    packet.write_to(&mut writer).unwrap();
    assert_eq!(&writer.into_inner()[..], b"KAZAHANE 1.0.0\x01\x05\x00world");
}

#[test]
fn hello_response() {
    let p1 = HelloResponsePacket::new(HelloResponseStatusCode::OK, "");
    assert_eq!(p1.message_size, 0);
    assert_eq!(p1.message, b"");

    let p2 = HelloResponsePacket::new(HelloResponseStatusCode::Denied, "err");
    assert_eq!(p2.message_size, 3);
    assert_eq!(p2.message, b"err");
}
