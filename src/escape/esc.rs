use escape::EncodeEscape;
use num::{self, ToPrimitive};
use std;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Esc {
    Unspecified {
        intermediate: Option<u8>,
        /// The final character in the Escape sequence; this typically
        /// defines how to interpret the other parameters.
        control: u8,
    },
    Code(EscCode),
}

macro_rules! esc {
    ($low:expr) => {
        ($low as isize)
    };
    ($high:expr, $low:expr) => {
        ((($high as isize) << 8) | ($low as isize))
    };
}

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, Copy)]
pub enum EscCode {
    /// RIS - Full Reset
    FullReset = esc!('c'),
    /// IND - Index.  Note that for Vt52 and Windows 10 ANSI consoles,
    /// this is interpreted as CursorUp
    Index = esc!('D'),
    /// NEL - Next Line
    NextLine = esc!('E'),
    /// Move the cursor to the bottom left corner of the screen
    CursorPositionLowerLeft = esc!('F'),
    /// HTS - Horizontal Tab Set
    HorizontalTabSet = esc!('H'),
    /// RI - Reverse Index – Performs the reverse operation of \n, moves cursor up one line,
    /// maintains horizontal position, scrolls buffer if necessary
    ReverseIndex = esc!('M'),
    /// SS2 Single shift of G2 character set affects next character only
    SingleShiftG2 = esc!('N'),
    /// SS3 Single shift of G3 character set affects next character only
    SingleShiftG3 = esc!('O'),
    /// SPA - Start of Guarded Area
    StartOfGuardedArea = esc!('V'),
    /// EPA - End of Guarded Area
    EndOfGuardedArea = esc!('W'),
    /// SOS - Start of String
    StartOfString = esc!('X'),
    /// DECID - Return Terminal ID (obsolete form of CSI c - aka DA)
    ReturnTerminalId = esc!('Z'),
    /// ST - String Terminator
    StringTerminator = esc!('\\'),
    /// PM - Privacy Message
    PrivacyMessage = esc!('^'),
    /// APC - Application Program Command
    ApplicationProgramCommand = esc!('_'),

    /// DECSC - Save cursor position
    DecSaveCursorPosition = esc!('7'),
    /// DECSR - Restore saved cursor position
    DecRestoreCursorPosition = esc!('8'),
    /// DECPAM - Application Keypad
    DecApplicationKeyPad = esc!('='),
    /// DECPNM - Normal Keypad
    DecNormalKeyPad = esc!('>'),

    /// Designate Character Set – DEC Line Drawing
    DecLineDrawing = esc!('(', '0'),
    /// Designate Character Set – US ASCII
    AsciiCharacterSet = esc!('(', 'B'),

    /// These are typically sent by the terminal when keys are pressed
    ApplicationModeArrowUpPress = esc!('O', 'A'),
    ApplicationModeArrowDownPress = esc!('O', 'B'),
    ApplicationModeArrowRightPress = esc!('O', 'C'),
    ApplicationModeArrowLeftPress = esc!('O', 'D'),
    ApplicationModeHomePress = esc!('O', 'H'),
    ApplicationModeEndPress = esc!('O', 'F'),
    F1Press = esc!('O', 'P'),
    F2Press = esc!('O', 'Q'),
    F3Press = esc!('O', 'R'),
    F4Press = esc!('O', 'S'),
}

impl Esc {
    pub fn parse(intermediate: Option<u8>, control: u8) -> Self {
        Self::internal_parse(intermediate, control).unwrap_or_else(|_| Esc::Unspecified {
            intermediate,
            control,
        })
    }

    fn internal_parse(intermediate: Option<u8>, control: u8) -> Result<Self, ()> {
        let packed = match intermediate {
            Some(high) => ((high as u16) << 8) | control as u16,
            None => control as u16,
        };

        let code = num::FromPrimitive::from_u16(packed).ok_or(())?;

        Ok(Esc::Code(code))
    }
}

impl EncodeEscape for Esc {
    // TODO: data size optimization opportunity: if we could somehow know that we
    // had a run of CSI instances being encoded in sequence, we could
    // potentially collapse them together.  This is a few bytes difference in
    // practice so it may not be worthwhile with modern networks.
    fn encode_escape<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        w.write_all(&[0x1b])?;
        use self::Esc::*;
        match self {
            Code(code) => {
                let packed = code.to_u16()
                    .expect("num-derive failed to implement ToPrimitive");
                if packed > u8::max_value() as u16 {
                    let buf = [(packed >> 8) as u8, (packed & 0xff) as u8];
                    w.write_all(&buf)?;
                } else {
                    let buf = [(packed & 0xff) as u8];
                    w.write_all(&buf)?;
                }
            }
            Unspecified {
                intermediate,
                control,
            } => {
                if let Some(i) = intermediate {
                    let buf = [*i, *control];
                    w.write_all(&buf)?;
                } else {
                    let buf = [*control];
                    w.write_all(&buf)?;
                }
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn encode(osc: &Esc) -> String {
        let mut res = Vec::new();
        osc.encode_escape(&mut res).unwrap();
        String::from_utf8(res).unwrap()
    }

    fn parse(esc: &str) -> Esc {
        let result = if esc.len() == 1 {
            Esc::parse(None, esc.as_bytes()[0])
        } else {
            Esc::parse(Some(esc.as_bytes()[0]), esc.as_bytes()[1])
        };

        assert_eq!(encode(&result), format!("\x1b{}", esc));

        result
    }

    #[test]
    fn test() {
        assert_eq!(parse("(0"), Esc::Code(EscCode::DecLineDrawing));
        assert_eq!(parse("(B"), Esc::Code(EscCode::AsciiCharacterSet));
    }
}
