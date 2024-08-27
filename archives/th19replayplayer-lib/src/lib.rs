use std::io::{BufRead, Write};

use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use junowen_lib::{
    structs::{
        selection::{Difficulty, PlayerMatchup},
        settings::GameSettings,
    },
    Th19,
};

pub enum FileInputList {
    HumanVsHuman(Vec<(u16, u16)>),
    HumanVsCpu(Vec<u16>),
}

impl Default for FileInputList {
    fn default() -> Self {
        Self::HumanVsHuman(Vec::new())
    }
}

#[derive(Default)]
pub struct ReplayFile {
    pub rand_seed1: u32,
    pub rand_seed2: u32,
    pub difficulty: Difficulty,
    pub player_matchup: PlayerMatchup,
    pub battle_settings: GameSettings,
    pub p1_character: u8,
    pub p1_card: u8,
    pub p2_character: u8,
    pub p2_card: u8,
    pub inputs: FileInputList,
}

impl ReplayFile {
    pub fn read_header_from_memory(th19: &Th19) -> Result<Self> {
        let player_matchup = th19.selection().player_matchup;
        Ok(Self {
            rand_seed1: th19.rand_seed1()?,
            rand_seed2: th19.rand_seed2()?,
            difficulty: th19.selection().difficulty,
            player_matchup,
            battle_settings: th19.game_settings_in_game()?,
            p1_character: th19.selection().p1().character as u8,
            p1_card: th19.selection().p1().card as u8,
            p2_character: th19.selection().p2().character as u8,
            p2_card: th19.selection().p2().card as u8,
            inputs: if player_matchup == PlayerMatchup::HumanVsHuman
                || player_matchup == PlayerMatchup::YoukaiVsYoukai
            {
                FileInputList::HumanVsHuman(Vec::new())
            } else {
                FileInputList::HumanVsCpu(Vec::new())
            },
        })
    }

    pub fn read_from_reader(reader: &mut impl BufRead) -> Result<Self> {
        let mut buf = BytesMut::new();
        buf.resize(13, 0);
        reader.read_exact(&mut buf)?;
        let rand_seed1 = buf.get_u32_le();
        let rand_seed2 = buf.get_u32_le();
        let difficulty = Difficulty::try_from(buf.get_u8() as u32)?;
        let player_matchup = PlayerMatchup::try_from(buf.get_u8() as u32)?;
        let battle_settings = GameSettings::new(
            buf.get_u8() as u32,
            buf.get_u8() as u32,
            buf.get_u8() as u32,
        );
        let p1_character = buf.get_u8();
        let p1_card = buf.get_u8();
        let p2_character = buf.get_u8();
        let p2_card = buf.get_u8();

        let inputs = match player_matchup {
            PlayerMatchup::HumanVsHuman
            | PlayerMatchup::CpuVsCpu
            | PlayerMatchup::YoukaiVsYoukai => {
                let mut vec = Vec::new();
                loop {
                    if reader.fill_buf()?.is_empty() {
                        break;
                    }
                    buf.clear();
                    buf.resize(4, 0);
                    reader.read_exact(&mut buf)?;
                    vec.push((buf.get_u16_le(), buf.get_u16_le()));
                }
                FileInputList::HumanVsHuman(vec)
            }
            PlayerMatchup::HumanVsCpu => {
                let mut vec = Vec::new();
                loop {
                    if reader.fill_buf()?.is_empty() {
                        break;
                    }
                    buf.clear();
                    buf.resize(2, 0);
                    reader.read_exact(&mut buf)?;
                    vec.push(buf.get_u16_le());
                }
                FileInputList::HumanVsCpu(vec)
            }
        };
        Ok(Self {
            rand_seed1,
            rand_seed2,
            difficulty,
            player_matchup,
            battle_settings,
            p1_character,
            p1_card,
            p2_character,
            p2_card,
            inputs,
        })
    }

    pub fn write_to(&self, writer: &mut impl Write) -> Result<()> {
        let mut buf = BytesMut::new();
        buf.put_u32_le(self.rand_seed1);
        buf.put_u32_le(self.rand_seed2);
        buf.put_u8(self.difficulty as u8);
        buf.put_u8(self.player_matchup as u8);
        buf.put_u8(self.battle_settings.common() as u8);
        buf.put_u8(self.battle_settings.p1() as u8);
        buf.put_u8(self.battle_settings.p2() as u8);
        buf.put_u8(self.p1_character);
        buf.put_u8(self.p1_card);
        buf.put_u8(self.p2_character);
        buf.put_u8(self.p2_card);
        writer.write_all(&buf)?;
        buf.clear();

        match &self.inputs {
            FileInputList::HumanVsHuman(vec) => {
                for input in vec {
                    buf.put_u16_le(input.0);
                    buf.put_u16_le(input.1);
                    writer.write_all(&buf)?;
                    buf.clear();
                }
            }
            FileInputList::HumanVsCpu(vec) => {
                for input in vec {
                    buf.put_u16_le(*input);
                    writer.write_all(&buf)?;
                    buf.clear();
                }
            }
        }
        Ok(())
    }
}
