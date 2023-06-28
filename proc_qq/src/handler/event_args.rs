use ricq_core::msg::elem::RQElem;

use crate::FriendMessageEvent;
use crate::GroupMessageEvent;
use crate::GroupTempMessageEvent;
use crate::MessageEvent;
use crate::{ImageElement, MessageContentTrait};

#[derive(Clone, Debug)]
pub enum EventArg {
    All(Vec<EventArg>),
    Any(Vec<EventArg>),
    Not(Vec<EventArg>),
    Regexp(String),
    Eq(String),
    TrimRegexp(String),
    TrimEq(String),
}

#[derive(Clone)]
pub enum HandEvent<'a> {
    MessageEvent(&'a MessageEvent, String),
    FriendMessageEvent(&'a FriendMessageEvent, String),
    GroupMessageEvent(&'a GroupMessageEvent, String),
    GroupTempMessageEvent(&'a GroupTempMessageEvent, String),
}

impl HandEvent<'_> {
    pub fn content(&self) -> ::anyhow::Result<&'_ String> {
        Ok(match self {
            HandEvent::MessageEvent(_, content) => &content,
            HandEvent::FriendMessageEvent(_, content) => &content,
            HandEvent::GroupMessageEvent(_, content) => &content,
            HandEvent::GroupTempMessageEvent(_, content) => &content,
        })
    }
}

impl<'a> From<&'a MessageEvent> for HandEvent<'a> {
    fn from(value: &'a MessageEvent) -> Self {
        Self::MessageEvent(value, value.message_content())
    }
}

impl<'a> From<&'a FriendMessageEvent> for HandEvent<'a> {
    fn from(value: &'a FriendMessageEvent) -> Self {
        Self::FriendMessageEvent(value, value.message_content())
    }
}

impl<'a> From<&'a GroupMessageEvent> for HandEvent<'a> {
    fn from(value: &'a GroupMessageEvent) -> Self {
        Self::GroupMessageEvent(value, value.message_content())
    }
}

impl<'a> From<&'a GroupTempMessageEvent> for HandEvent<'a> {
    fn from(value: &'a GroupTempMessageEvent) -> Self {
        Self::GroupTempMessageEvent(value, value.message_content())
    }
}

pub fn match_event_args_all(args: Vec<EventArg>, event: HandEvent) -> ::anyhow::Result<bool> {
    for x in args {
        if !match_event_item(x, event.clone())? {
            return Ok(false);
        }
    }
    // 一个条件都没有认为是true
    Ok(true)
}

fn match_event_args_any(args: Vec<EventArg>, event: HandEvent) -> ::anyhow::Result<bool> {
    for x in args {
        if match_event_item(x, event.clone())? {
            return Ok(true);
        }
    }
    // 一个条件都没有认为是false
    Ok(false)
}

fn match_event_args_not(args: Vec<EventArg>, event: HandEvent) -> ::anyhow::Result<bool> {
    for x in args {
        if match_event_item(x, event.clone())? {
            return Ok(false);
        }
    }
    // 一个条件都没有认为是true
    Ok(true)
}

fn match_event_args_regexp(args: String, event: HandEvent) -> ::anyhow::Result<bool> {
    Ok(regex::Regex::new(args.as_str())?.is_match(event.content()?.as_str()))
}

fn match_event_args_eq(args: String, event: HandEvent) -> ::anyhow::Result<bool> {
    Ok(args.eq(event.content()?.as_str()))
}

fn match_event_args_trim_regexp(args: String, event: HandEvent) -> ::anyhow::Result<bool> {
    Ok(regex::Regex::new(args.as_str())?.is_match(event.content()?.trim()))
}

fn match_event_args_trim_eq(args: String, event: HandEvent) -> ::anyhow::Result<bool> {
    Ok(args.eq(event.content()?.trim()))
}

fn match_event_item(arg: EventArg, event: HandEvent) -> ::anyhow::Result<bool> {
    match arg {
        EventArg::All(v) => match_event_args_all(v, event.clone()),
        EventArg::Any(v) => match_event_args_any(v, event.clone()),
        EventArg::Not(v) => match_event_args_not(v, event.clone()),
        EventArg::Regexp(v) => match_event_args_regexp(v, event.clone()),
        EventArg::Eq(v) => match_event_args_eq(v, event.clone()),
        EventArg::TrimRegexp(v) => match_event_args_trim_regexp(v, event.clone()),
        EventArg::TrimEq(v) => match_event_args_trim_eq(v, event.clone()),
    }
}

//

pub struct CommandMatcher {
    pub idx: usize,
    pub elements: Vec<RQElem>,
    pub matching: String,
}

impl CommandMatcher {
    pub fn new(value: Vec<RQElem>) -> CommandMatcher {
        let mut matcher = CommandMatcher {
            idx: 0,
            elements: value,
            matching: String::new(),
        };
        matcher.push_text();
        matcher
    }

    pub fn push_text(&mut self) {
        loop {
            if self.idx >= self.elements.len() {
                break;
            }
            let ele: &RQElem = self.elements.get(self.idx).unwrap();
            match ele {
                RQElem::Text(st) => {
                    self.matching.push_str(st.content.as_str());
                    self.idx += 1;
                }
                RQElem::Other(_) => {
                    self.idx += 1;
                }
                _ => break,
            }
        }
        let build = self.matching.trim().to_string();
        self.matching = build;
    }

    pub fn match_command(&mut self, command_name: &str) -> bool {
        let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
        let mut sp = sp_regexp.split(self.matching.as_str());
        if let Some(first) = sp.next() {
            if command_name.eq(first) {
                self.matching = self.matching[first.len()..].trim().to_string();
                return true;
            }
        }
        return false;
    }

    pub fn not_blank(&self) -> bool {
        !self.matching.is_empty() || self.idx < self.elements.len()
    }

    pub fn tuple_matcher(&mut self) -> Option<TupleMatcher> {
        if !self.matching.is_empty() {
            let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
            let mut sp = sp_regexp.split(self.matching.as_str());
            if let Some(first) = sp.next() {
                let first = first.to_string();
                self.matching = self.matching[first.len()..].trim().to_string();
                return Some(TupleMatcher::new(first));
            }
        }
        None
    }
}

pub trait FromCommandMatcher: Sized {
    fn get(s: &mut CommandMatcher) -> Option<Self>;
}

#[inline]
pub fn matcher_get<F: Sized + FromCommandMatcher>(matcher: &mut CommandMatcher) -> Option<F> {
    F::get(matcher)
}

impl FromCommandMatcher for String {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        if matcher.matching.is_empty() {
            return None;
        }
        let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
        let mut sp = sp_regexp.split(matcher.matching.as_str());
        if let Some(first) = sp.next() {
            let result = Some(first.to_string());
            matcher.matching = matcher.matching[first.len()..].trim().to_string();
            return result;
        }
        None
    }
}

impl FromCommandMatcher for Option<String> {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        let mut result = None;
        if matcher.matching.is_empty() {
            return Some(result);
        }
        let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
        let mut sp = sp_regexp.split(matcher.matching.as_str());
        if let Some(first) = sp.next() {
            result = Some(first.to_string());
            matcher.matching = matcher.matching[first.len()..].trim().to_string();
        }
        Some(result)
    }
}

impl FromCommandMatcher for Vec<String> {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
        let result = sp_regexp
            .split(matcher.matching.as_str())
            .map(String::from)
            .collect();
        matcher.matching = String::default();
        Some(result)
    }
}

macro_rules! command_base_ty_supplier {
    ($ty:ty) => {
        impl FromCommandMatcher for $ty {
            fn get(matcher: &mut CommandMatcher) -> Option<$ty> {
                if matcher.matching.is_empty() {
                    return None;
                }
                let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
                let mut sp = sp_regexp.split(matcher.matching.as_str());
                if let Some(first) = sp.next() {
                    let result = match first.parse::<$ty>() {
                        Ok(value) => Some(value),
                        Err(_) => return None,
                    };
                    matcher.matching = matcher.matching[first.len()..].trim().to_string();
                    return result;
                }
                None
            }
        }

        impl FromCommandMatcher for Option<$ty> {
            fn get(matcher: &mut CommandMatcher) -> Option<Self> {
                let mut result = None;
                if matcher.matching.is_empty() {
                    return Some(result);
                }
                let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
                let mut sp = sp_regexp.split(matcher.matching.as_str());
                if let Some(first) = sp.next() {
                    match first.parse::<$ty>() {
                        Ok(value) => {
                            result = Some(value);
                            matcher.matching = matcher.matching[first.len()..].trim().to_string();
                        }
                        _ => {}
                    };
                }
                return Some(result);
            }
        }

        impl FromCommandMatcher for Vec<$ty> {
            fn get(matcher: &mut CommandMatcher) -> Option<Self> {
                let mut result = vec![];
                if matcher.matching.is_empty() {
                    return Some(result);
                }
                let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
                let sp = sp_regexp.split(matcher.matching.as_str());
                let mut new_matching = vec![];
                for x in sp {
                    if !new_matching.is_empty() {
                        new_matching.push(x);
                    } else {
                        match x.parse::<$ty>() {
                            Ok(value) => result.push(value),
                            Err(_) => {
                                if result.is_empty() {
                                    return Some(result);
                                } else {
                                    new_matching.push(x);
                                }
                            }
                        }
                    }
                }
                matcher.matching = new_matching.join(" ");
                Some(result)
            }
        }
    };
}

command_base_ty_supplier!(i8);
command_base_ty_supplier!(u8);
command_base_ty_supplier!(i16);
command_base_ty_supplier!(u16);
command_base_ty_supplier!(i32);
command_base_ty_supplier!(u32);
command_base_ty_supplier!(i64);
command_base_ty_supplier!(u64);
command_base_ty_supplier!(i128);
command_base_ty_supplier!(u128);
command_base_ty_supplier!(isize);
command_base_ty_supplier!(usize);
command_base_ty_supplier!(f32);
command_base_ty_supplier!(f64);
command_base_ty_supplier!(bool);
command_base_ty_supplier!(char);

macro_rules! command_rq_element_ty_supplier {
    ($ty:ty, $mat:path) => {
        impl FromCommandMatcher for $ty {
            fn get(matcher: &mut CommandMatcher) -> Option<Self> {
                if !matcher.matching.is_empty() {
                    return None;
                }
                if matcher.idx >= matcher.elements.len() {
                    return None;
                }
                if let $mat(item) = matcher.elements.get(matcher.idx).unwrap() {
                    let result = Some(item.clone());
                    matcher.idx += 1;
                    matcher.push_text();
                    return result;
                }
                None
            }
        }

        impl FromCommandMatcher for Option<$ty> {
            fn get(matcher: &mut CommandMatcher) -> Option<Self> {
                let mut result = None;
                if !matcher.matching.is_empty() {
                    return Some(result);
                }
                if matcher.idx >= matcher.elements.len() {
                    return Some(result);
                }
                if let $mat(item) = matcher.elements.get(matcher.idx).unwrap() {
                    result = Some(item.clone());
                    matcher.idx += 1;
                    matcher.push_text();
                }
                Some(result)
            }
        }

        impl FromCommandMatcher for Vec<$ty> {
            fn get(matcher: &mut CommandMatcher) -> Option<Self> {
                let mut result = vec![];
                if !matcher.matching.is_empty() {
                    return Some(result);
                }
                loop {
                    if matcher.idx >= matcher.elements.len() {
                        break;
                    }
                    if let $mat(item) = matcher.elements.get(matcher.idx).unwrap() {
                        result.push(item.clone());
                        matcher.idx += 1;
                        matcher.push_text();
                    } else {
                        break;
                    }
                }
                Some(result)
            }
        }
    };
}

command_rq_element_ty_supplier!(ricq::msg::elem::At, RQElem::At);
command_rq_element_ty_supplier!(ricq::msg::elem::Face, RQElem::Face);
command_rq_element_ty_supplier!(ricq::msg::elem::MarketFace, RQElem::MarketFace);
command_rq_element_ty_supplier!(ricq::msg::elem::Dice, RQElem::Dice);
command_rq_element_ty_supplier!(ricq::msg::elem::FingerGuessing, RQElem::FingerGuessing);
command_rq_element_ty_supplier!(ricq::msg::elem::LightApp, RQElem::LightApp);
command_rq_element_ty_supplier!(ricq::msg::elem::RichMsg, RQElem::RichMsg);
command_rq_element_ty_supplier!(ricq::msg::elem::FriendImage, RQElem::FriendImage);
command_rq_element_ty_supplier!(ricq::msg::elem::GroupImage, RQElem::GroupImage);
command_rq_element_ty_supplier!(ricq::msg::elem::FlashImage, RQElem::FlashImage);
command_rq_element_ty_supplier!(ricq::msg::elem::VideoFile, RQElem::VideoFile);

impl FromCommandMatcher for ImageElement {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        if !matcher.matching.is_empty() {
            return None;
        }
        if matcher.idx >= matcher.elements.len() {
            return None;
        }
        match matcher.elements.get(matcher.idx).unwrap() {
            RQElem::FriendImage(image) => {
                let result = Some(ImageElement::FriendImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
                result
            }
            RQElem::GroupImage(image) => {
                let result = Some(ImageElement::GroupImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
                result
            }
            RQElem::FlashImage(image) => {
                let result = Some(ImageElement::FlashImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
                result
            }
            _ => None,
        }
    }
}

impl FromCommandMatcher for Option<ImageElement> {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        let mut result = None;
        if !matcher.matching.is_empty() {
            return Some(result);
        }
        if matcher.idx >= matcher.elements.len() {
            return Some(result);
        }
        match matcher.elements.get(matcher.idx).unwrap() {
            RQElem::FriendImage(image) => {
                result = Some(ImageElement::FriendImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
            }
            RQElem::GroupImage(image) => {
                result = Some(ImageElement::GroupImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
            }
            RQElem::FlashImage(image) => {
                result = Some(ImageElement::FlashImage(image.clone()));
                matcher.idx += 1;
                matcher.push_text();
            }
            _ => (),
        }
        Some(result)
    }
}

impl FromCommandMatcher for Vec<ImageElement> {
    fn get(matcher: &mut CommandMatcher) -> Option<Self> {
        let mut result = vec![];
        if !matcher.matching.is_empty() {
            return Some(result);
        }
        loop {
            if matcher.idx >= matcher.elements.len() {
                break;
            }
            match matcher.elements.get(matcher.idx).unwrap() {
                RQElem::FriendImage(image) => {
                    result.push(ImageElement::FriendImage(image.clone()));
                    matcher.idx += 1;
                    matcher.push_text();
                }
                RQElem::GroupImage(image) => {
                    result.push(ImageElement::GroupImage(image.clone()));
                    matcher.idx += 1;
                    matcher.push_text();
                }
                RQElem::FlashImage(image) => {
                    result.push(ImageElement::FlashImage(image.clone()));
                    matcher.idx += 1;
                    matcher.push_text();
                }
                _ => break,
            }
        }
        Some(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TupleMatcherElement {
    Command(&'static str),
    Param,
    Enum(Vec<&'static str>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleMatcher(String);

impl TupleMatcher {
    pub fn new(context: String) -> Self {
        Self(context)
    }
}

impl TupleMatcher {
    pub fn match_command(&mut self, command: &str) -> bool {
        if let Some(idx) = self.0.find(command) {
            if idx == 0 {
                self.0 = self.0[command.len()..].to_string();
                return true;
            }
        }
        false
    }
}

pub trait FromTupleMatcher: Sized {
    fn get(matcher: &mut TupleMatcher) -> Option<Self>;
}

#[inline]
pub fn tuple_matcher_get<F: Sized + FromTupleMatcher>(matcher: &mut TupleMatcher) -> Option<F> {
    F::get(matcher)
}

impl FromTupleMatcher for String {
    fn get(matcher: &mut TupleMatcher) -> Option<Self> {
        if matcher.0.is_empty() {
            None
        } else {
            let result = Some(matcher.0.clone());
            matcher.0 = "".to_string();
            result
        }
    }
}

impl FromTupleMatcher for Option<String> {
    fn get(matcher: &mut TupleMatcher) -> Option<Self> {
        if matcher.0.is_empty() {
            Some(None)
        } else {
            let result = Some(matcher.0.clone());
            matcher.0 = "".to_string();
            Some(result)
        }
    }
}

impl FromTupleMatcher for Vec<String> {
    fn get(matcher: &mut TupleMatcher) -> Option<Self> {
        if matcher.0.is_empty() {
            Some(vec![])
        } else {
            let result = Some(vec![matcher.0.clone()]);
            matcher.0 = "".to_string();
            result
        }
    }
}

impl FromTupleMatcher for Vec<Option<String>> {
    fn get(matcher: &mut TupleMatcher) -> Option<Self> {
        if matcher.0.is_empty() {
            Some(vec![])
        } else {
            let result = Some(vec![Some(matcher.0.clone())]);
            matcher.0 = "".to_string();
            result
        }
    }
}

//todo 浮点类型要分开处理了

macro_rules! tuple_base_ty_supplier {
    ($ty:ty, $regexp:expr) => {
        impl FromTupleMatcher for $ty {
            fn get(matcher: &mut TupleMatcher) -> Option<Self> {
                let regex = regex::Regex::new($regexp).expect("proc_qq 的正则错误");
                if let Some(find) = regex.find(matcher.0.as_str()) {
                    if find.start() == 0 {
                        matcher.0 = matcher.0.as_str()[find.start()..].to_string();
                    }
                    return matcher.0.parse::<$ty>().ok();
                }
                None
            }
        }

        impl FromTupleMatcher for Option<$ty> {
            fn get(matcher: &mut TupleMatcher) -> Option<Self> {
                Some(tuple_matcher_get::<$ty>(matcher))
            }
        }

        impl FromTupleMatcher for Vec<$ty> {
            fn get(matcher: &mut TupleMatcher) -> Option<Self> {
                Some(match tuple_matcher_get::<$ty>(matcher) {
                    None => vec![],
                    Some(value) => vec![value],
                })
            }
        }
    };
}

macro_rules! tuple_base_ty_supplier_ix {
    ($ty:ty) => {
        tuple_base_ty_supplier!($ty, r#"-?\d+"#);
    };
}

macro_rules! tuple_base_ty_supplier_ux {
    ($ty:ty) => {
        tuple_base_ty_supplier!($ty, r#"\d+"#);
    };
}

macro_rules! tuple_base_ty_supplier_fx {
    ($ty:ty) => {
        tuple_base_ty_supplier!($ty, r#"-?\d+(\.\d+)"#);
    };
}

macro_rules! tuple_base_ty_supplier_ix_types {
    ($($ty:ty),*) => {
        $(tuple_base_ty_supplier_ix!($ty);)*
    };
}

macro_rules! tuple_base_ty_supplier_ux_types {
    ($($ty:ty),*) => {
        $(tuple_base_ty_supplier_ux!($ty);)*
    };
}

macro_rules! tuple_base_ty_supplier_fx_types {
    ($($ty:ty),*) => {
        $(tuple_base_ty_supplier_fx!($ty);)*
    };
}

tuple_base_ty_supplier_ix_types!(i8, i16, i32, i64, i128, isize, char);
tuple_base_ty_supplier_ux_types!(u8, u16, u32, u64, u128, usize);
tuple_base_ty_supplier_fx_types!(f32, f64);
tuple_base_ty_supplier!(bool, "(true)|(false)");

// enums

pub trait TryFromStr: Sized {
    fn try_from(value: &str) -> Result<Self, anyhow::Error>;
}

impl TryFromStr for String {
    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        Ok(value.to_string())
    }
}

macro_rules! enum_try_from_str {
    ($($ty:ty),*) => {
        $(impl TryFromStr for $ty {
            fn try_from(value: &str) -> Result<Self, anyhow::Error> {
                Ok(value.parse::<$ty>()?)
            }
        })*
    };
}

enum_try_from_str!(i8, i16, i32, i64, i128, isize, char);
enum_try_from_str!(u8, u16, u32, u64, u128, usize);
enum_try_from_str!(f32, f64);
enum_try_from_str!(bool);

#[inline]
pub fn matcher_get_enum<'a, F: Sized + TryFromStr>(
    matcher: &mut CommandMatcher,
    values: Vec<&str>,
) -> Option<F> {
    if matcher.matching.is_empty() {
        return None;
    }
    let sp_regexp = regex::Regex::new("\\s+").expect("proc_qq 正则错误");
    let mut sp = sp_regexp.split(matcher.matching.as_str());
    if let Some(first) = sp.next() {
        if values.contains(&first) {
            let result = match F::try_from(first) {
                Ok(v) => Some(v),
                Err(_) => return None,
            };
            matcher.matching = matcher.matching[first.len()..].trim().to_string();
            return result;
        }
    }
    None
}

#[inline]
pub fn tuple_matcher_get_enum<'a, F: Sized + TryFromStr>(
    matcher: &mut TupleMatcher,
    values: Vec<&str>,
) -> Option<F> {
    if matcher.0.is_empty() {
        return None;
    }
    for x in values {
        if matcher.0.starts_with(x) {
            return match F::try_from(x) {
                Ok(value) => {
                    matcher.0 = matcher.0[x.len()..].trim().to_string();
                    Some(value)
                }
                Err(_) => None,
            };
        }
    }
    None
}
