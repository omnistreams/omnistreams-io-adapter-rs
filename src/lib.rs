use std::io::Write;
use std::collections::VecDeque;
//use std::{thread, time};


#[derive(Debug, PartialEq)]
enum ConsumerEvent {
    Request(usize),
    Termination,
    Finish,
}

#[derive(Debug, PartialEq)]
enum ConsumerError {
    WriteWithoutRequest,
}

trait Consumer {
    fn write(&mut self, data: &[u8]) -> Result<(), ConsumerError>;
    fn emit(&mut self, event: ConsumerEvent);
    fn next_event(&mut self) -> Option<ConsumerEvent>;
    fn update(&mut self);
}


pub struct WriteAdapterConsumer<'a> {
    writer: Box<'a + Write>,
    demand: usize,
    event_queue: VecDeque<ConsumerEvent>,
    buffered: Option<Vec<u8>>,
}

impl<'a> WriteAdapterConsumer<'a> {
    pub fn new<T: 'a + Write>(writer: T) -> WriteAdapterConsumer<'a> {

        let initial_demand = 1;

        let mut consumer = WriteAdapterConsumer {
            writer: Box::new(writer),
            demand: initial_demand,
            event_queue: VecDeque::new(),
            buffered: None,
        };

        consumer.emit(ConsumerEvent::Request(initial_demand));

        consumer
    }

}

impl<'a> Consumer for WriteAdapterConsumer<'a> {
    fn write(&mut self, data: &[u8]) -> Result<(), ConsumerError> {
        if self.demand > 0 {
            // TODO: handle case where only partial data is written
            match self.writer.write(data) {
                Ok(n) => {
                    if n != data.len() {
                        self.buffered = Some(data[n..].into());
                        self.demand -= 1;
                    }
                    else {
                        self.emit(ConsumerEvent::Request(1));
                    }

                    Ok(())
                },
                Err(_) => {
                    println!("getting buffed");
                    self.buffered = Some(data.into());
                    self.demand -= 1;
                    Ok(())
                },
            }
        }
        else {
            Err(ConsumerError::WriteWithoutRequest)
        }
    }

    fn emit(&mut self, event: ConsumerEvent) {
        self.event_queue.push_back(event);
    }

    fn next_event(&mut self) -> Option<ConsumerEvent> {
        self.event_queue.pop_front()
    }

    fn update(&mut self) {
    }
}


#[cfg(test)]
mod tests {

    use super::*;
    use std::fs::File;
    use std::io;
    use std::io::Cursor;

    struct FailWriter {
    }

    impl Write for FailWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::Other, "YOLO"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct PartialWriter {
    }

    impl Write for PartialWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Ok(1)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }


    #[test]
    fn write_without_request_fails() {
        let writer = FailWriter{};
        let mut consumer = WriteAdapterConsumer::new(writer);
        assert_eq!(consumer.write(&[65]), Ok(()));
        assert_eq!(consumer.write(&[65]), Err(ConsumerError::WriteWithoutRequest));
    }

    #[test]
    fn partial_write() {
        let writer = PartialWriter{};
        let mut consumer = WriteAdapterConsumer::new(writer);
        assert_eq!(consumer.write(&[65, 66]), Ok(()));
        assert_eq!(consumer.write(&[65]), Err(ConsumerError::WriteWithoutRequest));
    }

    #[test]
    fn new_emits_request() {
        let buf = Cursor::new(vec![0; 15]);
        let mut consumer = WriteAdapterConsumer::new(buf);
        let event = consumer.next_event().unwrap();
        assert_eq!(event, ConsumerEvent::Request(1));
    }

    #[test]
    fn buffer() {
        let buf = Cursor::new(vec![0; 15]);
        let mut consumer = WriteAdapterConsumer::new(buf);
        assert_eq!(consumer.write(&[65]), Ok(()));
        //assert_eq!(consumer.write(&[65]), Ok(()));
    }

    #[test]
    fn demand_decreases_on_write() {

        //let mut buf = Cursor::new(vec![0; 15]);
        //let mut consumer = WriteAdapterConsumer::new(buf);
        //assert_eq!(consumer.write(&[65]), Ok(()));
        //assert_eq!(consumer.write(&[65]), Err(ConsumerError::WriteWithoutRequest));
    }

    #[test]
    fn it_works() {
        let num_lines = 10;
        let mut num_written = 0;
        let mut file = File::create("test.txt").unwrap();
        let mut consumer = WriteAdapterConsumer::new(file);

        while num_written < num_lines {
            match consumer.next_event() {

                Some(event) => {
                    println!("{:?}", event);
                    match event {
                        ConsumerEvent::Request(n) => {
                            println!("Requested: {}", n);
                            assert_eq!(consumer.write(&[65, 67, 65, 67, 10]), Ok(()));
                            num_written += 1;
                        },
                        _ => {
                            panic!("Unhandled event: {:?}", event);
                        },
                    }
                },
                None => {
                    println!("no more events");
                    break;
                },
            }
        }
    }
}
