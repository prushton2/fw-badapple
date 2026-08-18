use std::time::Duration;

use serialport::SerialPort;

const COLS: usize = 9;
const ROWS: usize = 34;

// https://github.com/FrameworkComputer/inputmodule-rs/blob/main/commands.md
pub enum MatrixCommand {
    Brightness,
    Pattern,
    Bootloader,
    Sleep,
    GetSleep,
    Animate,
    GetAnimate,
    Panic,
    DrawBW,
    StageCol,
    FlushCols,
    SetText,
    StartGame,
    GameCtrl,
    GameStatus,
    SetColor,
    DisplayOn,
    InvertScreen,
    SetPxCol,
    FlushFB,
    Version,
}

impl From<MatrixCommand> for u8 {
    fn from(value: MatrixCommand) -> Self {
        match value {
            MatrixCommand::Brightness    =>  0x00,
            MatrixCommand::Pattern       =>  0x01,
            MatrixCommand::Bootloader    =>  0x02,
            MatrixCommand::Sleep         =>  0x03,
            MatrixCommand::GetSleep      =>  0x03,
            MatrixCommand::Animate       =>  0x04,
            MatrixCommand::GetAnimate    =>  0x04,
            MatrixCommand::Panic         =>  0x05,
            MatrixCommand::DrawBW        =>  0x06,
            MatrixCommand::StageCol      =>  0x07,
            MatrixCommand::FlushCols     =>  0x08,
            MatrixCommand::SetText       =>  0x09,
            MatrixCommand::StartGame     =>  0x10,
            MatrixCommand::GameCtrl      =>  0x11,
            MatrixCommand::GameStatus    =>  0x12,
            MatrixCommand::SetColor      =>  0x13,
            MatrixCommand::DisplayOn     =>  0x14,
            MatrixCommand::InvertScreen  =>  0x15,
            MatrixCommand::SetPxCol      =>  0x16,
            MatrixCommand::FlushFB       =>  0x17,
            MatrixCommand::Version       =>  0x20,
        }
    }
}

pub struct Matrix {
    port: Box<dyn SerialPort>,
    framebuffer: [[u8; ROWS]; COLS], // [col][row] -> brightness 0-255
}

impl Matrix {
    pub fn from_device_label(device_label: &str) -> Self {
        let port = serialport::new(device_label, 115_200)
            .timeout(Duration::from_millis(500))
            .open()
            .expect("failed to open port");
        Matrix {
            port: port,
            framebuffer: [[0; ROWS]; COLS]
        }
    }

    pub fn send_command(&mut self, cmd: MatrixCommand, params: &[u8]) -> std::io::Result<()> {
        let mut buf = vec![0x32, 0xAC, cmd.into()];
        buf.extend_from_slice(params);
        self.port.write_all(&buf)
    }

    pub fn write_buffer(&mut self, col: usize, row: usize, brightness: u8) {
        self.framebuffer[col][row] = brightness;
    }

    pub fn save_col(&mut self, col: usize) {
        // StageCol: [column_index, 34 brightness bytes for that column]
        let mut params = Vec::with_capacity(1 + ROWS);
        params.push(col as u8);
        params.extend_from_slice(&self.framebuffer[col]);
        let _ = self.send_command(MatrixCommand::StageCol, &params);
    }

    pub fn save_cols(&mut self) {
        for col in 0..COLS {
            self.save_col(col);
        }
    }

    pub fn clear_buffer(&mut self) {
        self.framebuffer = [[0; ROWS]; COLS];
    }
    
    pub fn flush_buffer(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(MatrixCommand::FlushCols, &[])?;
        Ok(())
    }
}
