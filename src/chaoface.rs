use crate::{
    chaoparam::{
        AL_EYE_NUM_ANGER, AL_EYE_NUM_DCHAOS, AL_EYE_NUM_HCHAOS, AL_EYE_NUM_NCHAOS, TYPE_D_POWER,
        TYPE_N_NORMAL,
    },
    ChaoParamGc,
};

const AL_EYE_TEXID_NORMAL: u16 = 0x0;
const AL_EYE_TEXID_KYA: u16 = 0x1;
const AL_EYE_TEXID_NAMU: u16 = 0x2;
const AL_EYE_TEXID_TOHOHO: u16 = 0x3;
const AL_EYE_TEXID_NIKO: u16 = 0x4;
const AL_EYE_TEXID_BIKKURI: u16 = 0x5;
const AL_EYE_TEXID_GURUGURU: u16 = 0x6;
const AL_EYE_TEXID_SUYASUYA: u16 = 0x7;
const AL_EYE_TEXID_DARK: u16 = 0x8;
const AL_EYE_TEXID_HERO: u16 = 0x9;
const AL_EYE_TEXID_NCHAOS: u16 = 10;
const AL_EYE_TEXID_HCHAOS: u16 = 11;
const AL_EYE_TEXID_DCHAOS: u16 = 12;

enum EyeColor {
    Neut,
    Hero,
    Dark,
    NChaos,
    HChaos,
    DChaos,
    Blue,
    Green,
    Red,
}

enum EyeLidBlinkMode {
    Open,
    Close,
    Wait,
    Stop,
}

pub struct ChaoFace {
    eyelid_close_ang: i32,
    eyelid_close_aim_ang: i32,

    eyelid_slope_ang: i32,
    eyelid_slope_aim_ang: i32,

    eyelid_blink_mode: EyeLidBlinkMode,
    eyelid_blink_ang: i32,

    eye_color: EyeColor,

    eye_curr_num: i8,
    eye_default_num: i8,

    pub eye_tex_id: u16,
    pub mouth_tex_id: [u16; 2],

    mouth_curr_num: i8,
    mouth_default_num: i8,
}

impl ChaoFace {
    pub fn init(chao_param: &ChaoParamGc) -> Self {
        let eye_color = match chao_param.chao_type {
            TYPE_N_NORMAL..=TYPE_D_POWER => match (chao_param.chao_type - TYPE_N_NORMAL) % 3 {
                1 => EyeColor::Hero,
                2 => EyeColor::Dark,
                _ => EyeColor::Neut,
            },
            crate::chaoparam::TYPE_N_CHAOS => EyeColor::NChaos,
            crate::chaoparam::TYPE_H_CHAOS => EyeColor::HChaos,
            crate::chaoparam::TYPE_D_CHAOS => EyeColor::DChaos,
            _ => EyeColor::Neut,
        };

        let (eye_num, mouth_num) = match chao_param.chao_type {
            crate::chaoparam::TYPE_N_CHAOS => (AL_EYE_NUM_NCHAOS, 0),
            crate::chaoparam::TYPE_H_CHAOS => (AL_EYE_NUM_HCHAOS, 0),
            crate::chaoparam::TYPE_D_CHAOS => (AL_EYE_NUM_DCHAOS, 0),
            _ => (
                if chao_param.is_dark() {
                    AL_EYE_NUM_ANGER
                } else {
                    chao_param.body.default_eye_num
                },
                chao_param.body.default_mouth_num,
            ),
        };

        let mut face = Self {
            eyelid_close_ang: 0,
            eyelid_close_aim_ang: 0,
            eyelid_slope_ang: 0,
            eyelid_slope_aim_ang: 0,
            eyelid_blink_mode: EyeLidBlinkMode::Open,
            eyelid_blink_ang: 0,
            eye_color,
            eye_curr_num: eye_num,
            eye_default_num: eye_num,
            eye_tex_id: 0,
            mouth_tex_id: [0, 0],
            mouth_curr_num: mouth_num,
            mouth_default_num: mouth_num,
        };

        face.set_eye(face.eye_curr_num);
        face.set_mouth(face.mouth_curr_num);

        face
    }

    pub fn get_eyelid_close_ang(&self) -> i32 {
        self.eyelid_blink_ang + self.eyelid_close_ang - 0x4000
    }

    pub fn get_eyelid_slope_ang(&self) -> i32 {
        self.eyelid_slope_ang
    }

    pub fn set_eye(&mut self, eye_id: i8) {
        self.eye_curr_num = eye_id;

        const TEX_ID_LIST: [u16; 14] = [
            AL_EYE_TEXID_NORMAL,
            AL_EYE_TEXID_KYA,
            AL_EYE_TEXID_NAMU,
            AL_EYE_TEXID_TOHOHO,
            AL_EYE_TEXID_NIKO,
            AL_EYE_TEXID_BIKKURI,
            AL_EYE_TEXID_GURUGURU,
            AL_EYE_TEXID_SUYASUYA,
            AL_EYE_TEXID_BIKKURI,
            AL_EYE_TEXID_NORMAL,
            AL_EYE_TEXID_NORMAL,
            AL_EYE_TEXID_NCHAOS,
            AL_EYE_TEXID_HCHAOS,
            AL_EYE_TEXID_DCHAOS,
        ];

        let color_tex_id = match self.eye_color {
            EyeColor::Neut => AL_EYE_TEXID_NORMAL,
            EyeColor::Hero => AL_EYE_TEXID_HERO,
            EyeColor::Dark => AL_EYE_TEXID_DARK,
            EyeColor::NChaos => AL_EYE_TEXID_NCHAOS,
            EyeColor::HChaos => AL_EYE_TEXID_HCHAOS,
            EyeColor::DChaos => AL_EYE_TEXID_DCHAOS,
            _ => AL_EYE_TEXID_NORMAL,
        };

        self.eye_tex_id = match self.eye_curr_num {
            crate::chaoparam::AL_EYE_NUM_NORMAL
            | crate::chaoparam::AL_EYE_NUM_TRON
            | crate::chaoparam::AL_EYE_NUM_ANGER => color_tex_id,
            _ => TEX_ID_LIST[self.eye_curr_num as usize],
        };
    }

    pub fn set_mouth(&mut self, mouth_id: i8) {
        const MOUTH_TEXID_LIST: [[u16; 2]; 13] = [
            [0, 0],
            [2, 1],
            [3, 0],
            [4, 0],
            [5, 0],
            [6, 0],
            [7, 0],
            [9, 8],
            [11, 10],
            [13, 12],
            [14, 0],
            [16, 15],
            [18, 17],
        ];

        self.mouth_curr_num = mouth_id;

        self.mouth_tex_id = MOUTH_TEXID_LIST[mouth_id as usize].clone();
    }
}
