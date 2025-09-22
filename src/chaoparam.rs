use std::path::PathBuf;

use zerocopy::*;

pub const TYPE_CHILD: i8 = 0x2;
pub const TYPE_N_NORMAL: i8 = 0x5;
pub const TYPE_H_NORMAL: i8 = 0x6;
pub const TYPE_D_NORMAL: i8 = 0x7;
pub const TYPE_N_SWIM: i8 = 0x8;
pub const TYPE_H_SWIM: i8 = 0x9;
pub const TYPE_D_SWIM: i8 = 0xA;
pub const TYPE_N_FLY: i8 = 0xB;
pub const TYPE_H_FLY: i8 = 0xC;
pub const TYPE_D_FLY: i8 = 0xD;
pub const TYPE_N_RUN: i8 = 0xE;
pub const TYPE_H_RUN: i8 = 0xF;
pub const TYPE_D_RUN: i8 = 0x10;
pub const TYPE_N_POWER: i8 = 0x11;
pub const TYPE_H_POWER: i8 = 0x12;
pub const TYPE_D_POWER: i8 = 0x13;
pub const TYPE_N_CHAOS: i8 = 0x14;
pub const TYPE_H_CHAOS: i8 = 0x15;
pub const TYPE_D_CHAOS: i8 = 0x16;
pub const TYPE_TAILS: i8 = 0x17;
pub const TYPE_KNUCKLES: i8 = 0x18;
pub const TYPE_AMY: i8 = 0x19;

pub const AL_EYE_NUM_NORMAL: i8 = 0x0;
pub const AL_EYE_NUM_KYA: i8 = 0x1;
pub const AL_EYE_NUM_NAMU: i8 = 0x2;
pub const AL_EYE_NUM_TOHOHO: i8 = 0x3;
pub const AL_EYE_NUM_NIKO: i8 = 0x4;
pub const AL_EYE_NUM_BIKKURI: i8 = 0x5;
pub const AL_EYE_NUM_GURUGURU: i8 = 0x6;
pub const AL_EYE_NUM_SUYASUYA: i8 = 0x7;
pub const AL_EYE_NUM_SHIROME: i8 = 0x8;
pub const AL_EYE_NUM_TRON: i8 = 0x9;
pub const AL_EYE_NUM_ANGER: i8 = 10;
pub const AL_EYE_NUM_NCHAOS: i8 = 11;
pub const AL_EYE_NUM_HCHAOS: i8 = 12;
pub const AL_EYE_NUM_DCHAOS: i8 = 13;
pub const AL_EYE_NUM_END: i8 = 0xE;

#[derive(Copy, Clone, FromBytes, IntoBytes)]
#[repr(C)]
pub struct ChaoId {
    gid: [u32; 2],
    id: [u32; 2],
    num: u32,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct RecordTime {
    minute: u8,
    second: u8,
    frame: u8,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoRacePersonalInfo {
    personal_record: [RecordTime; 10],
    nb_win: [i8; 10],
    medal_flag: u16,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoKaratePersonalInfo {
    rank: i8,
    level: i8,
    tournament: i8,
    padding: i8,
    nb_battle: u16,
    nb_win: u16,
    nb_lose: u16,
    nb_draw: u16,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoParts {
    minimal_flag: u32,
    minimal_parts: [i8; 8],
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoEmotion {
    flag: u16,
    mood_timer: u16,
    ill_timer: u16,
    timer: u16,
    mood: [i8; 8],
    state: [u16; 11],
    personality: [i8; 13],
    taste: i8,
    tv: i8,
    music: i8,
    ill_state: [i8; 6],
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoKnowledgePlayer {
    like: i8,
    fear: i8,
    distance: u16,
    meet: u16,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C, align(4))]
pub struct ChaoKnowledgeChao {
    id: ChaoId,
    like: i8,
    fear: i8,
    distance: u16,
    meet: i32,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoKnowledgeOther {
    like: u8,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoKnowledgeBattle {
    art_flag: i8,
    dance_flag: i8,
    song_flag: i8,
    music_flag: i8,
    stoy_flag: u16,
    ltoy_flag: u16,
    kw_timer: i32,
    player: [ChaoKnowledgePlayer; 6],
    chao: [ChaoKnowledgeChao; 20],
    bhv: [ChaoKnowledgeOther; 120],
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoGene {
    is_analyzed: i8,
    egg_color: i8,
    padding_1: i16,
    mother_id: ChaoId,
    father_id: ChaoId,
    mother_name: [i8; 8],
    father_name: [i8; 8],
    m_grandmother_name: [i8; 8],
    m_grandfather_name: [i8; 8],
    f_grandmother_name: [i8; 8],
    f_grandfather_name: [i8; 8],
    abl: [[i8; 2]; 8],
    life_time: [i8; 2],
    h_pos: [i8; 2],
    v_pos: [i8; 2],
    a_pos: [i8; 2],
    personality: [[i8; 2]; 13],
    taste: [i8; 2],
    tv: [i8; 2],
    music: [i8; 2],
    color: [i8; 2],
    non_tex: [i8; 2],
    jewel: [i8; 2],
    multi: [i8; 2],
    eye_pos: [i8; 2],
    eye_scl: [i8; 2],
    eye_ratio: [i8; 2],
    eye_color: [i8; 2],
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoBodyInfo {
    pub h_pos: f32,
    pub v_pos: f32,
    pub a_pos: f32,
    pub aim_h_pos: f32,
    pub aim_v_pos: f32,
    pub aim_a_pos: f32,
    pub growth: f32,
    pub eye_pos: f32,
    pub eye_scl: f32,
    pub eye_ratio: f32,
    pub eye_color: i8,
    pub default_eye_num: i8,
    pub default_mouth_num: i8,
    pub honbu_num: i8,
    pub honbu_color_num: i8,
    pub obake_head: i8,
    pub obake_body: i8,
    pub medal_num: i8,
    pub color_num: i8,
    pub non_tex: i8,
    pub jewel_num: i8,
    pub multi_num: i8,
    pub egg_color: i8,
    pub form_num: i8,
    pub form_subnum: i8,
    padding: i8,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct ChaoParamGc {
    pub gbachao: i8,
    pub gbaegg: i8,
    pub gbaberry: [i8; 8],
    pub padding0: i8,
    pub padding1: i8,
    pub gbaring: u32,
    pub boot_method: i8,
    pub birthplace: i8,
    pub name: [i8; 7],
    pub gbatype: i8,
    pub gbaskin: i8,
    pub gbamood: i8,
    pub gbabelly: i8,
    pub gbasleepy: i8,
    pub gbalonelyness: i8,
    pub padding2: i8,
    pub exp: [i8; 8],
    pub abl: [i8; 8],
    pub lev: [i8; 8],
    pub skill: [u16; 8],
    pub gbapalette: [u16; 16],
    pub rmsg: [u8; 16],
    pub runaway: u32,
    pub dummy: [i8; 4],
    pub chao_type: i8,
    pub place: i8,
    pub like: i16,
    pub class_num: i8,
    pub padding3: i8,
    pub age: u16,
    pub old: u16,
    pub life: u16,
    pub life_max: u16,
    pub nb_succeed: u16,
    pub chao_id: ChaoId,
    pub life_timer: u32,
    pub body: ChaoBodyInfo,
    race: ChaoRacePersonalInfo,
    padding4: i16,
    karate: ChaoKaratePersonalInfo,
    parts_btl: ChaoParts,
    emotion: ChaoEmotion,
    knowledge_btl: ChaoKnowledgeBattle,
    gene: ChaoGene,
}

impl Default for ChaoParamGc {
    fn default() -> Self {
        let bytes = [0u8; 64 + size_of::<ChaoParamGc>()];
        let mut param = Self::init_from_chao_file(&bytes);
        param.chao_type = TYPE_CHILD;
        param
    }
}

impl ChaoParamGc {
    pub fn init_from_chao_file(buf: &[u8]) -> Self {
        ChaoParamGc::read_from_bytes(&buf[64..(64 + size_of::<ChaoParamGc>())]).unwrap()
    }

    pub fn read_from_chao_file(path: &PathBuf) -> Self {
        let buf = std::fs::read(path).unwrap();
        ChaoParamGc::read_from_bytes(&buf[64..(64 + size_of::<ChaoParamGc>())]).unwrap()
    }

    pub fn is_dark(&self) -> bool {
        if self.chao_type < TYPE_D_NORMAL {
            return false;
        }

        ((self.chao_type - TYPE_N_NORMAL) % 3) == 2
    }
}
