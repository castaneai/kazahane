use binrw::binrw;
use tokio_util::codec::LengthDelimitedCodec;

const MAX_PACKET_SIZE: usize = 4096;

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

fn build_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .little_endian()
        .length_field_offset(15)
        .length_field_type::<u16>()
        .length_adjustment(15 + 2) // magic + len
        .num_skip(0)
        .max_frame_length(MAX_PACKET_SIZE)
        .new_codec()
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

#[tokio::test]
async fn packet_with_framed() {
    use std::io::Cursor;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
    };
    use tokio_stream::StreamExt;
    use tokio_util::codec::FramedRead;
    use binrw::BinRead;

    let listener = TcpListener::bind("localhost:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(b"KAZAHA").await.unwrap();
        client
            .write_all(b"NE 1.0.0\x01\x05\x00hello")
            .await
            .unwrap();
    });
    let (client, _) = listener.accept().await.unwrap();
    let mut reader = FramedRead::new(client, build_codec());
    let frame = reader.next().await;
    assert_eq!(frame.is_some(), true);
    let frame = frame.unwrap();
    assert_eq!(frame.is_ok(), true);
    let frame = frame.unwrap().freeze();
    let mut input = Cursor::new(frame.as_ref());
    let packet = KazahanePacket::read(&mut input).unwrap();
    assert_eq!(packet.payload, b"hello");
}
