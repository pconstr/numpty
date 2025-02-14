use crate::color::indexedcolor_from_avt;
use crate::color::truecolor_from_avt;
use ndarray::{Array2, Array3};


fn style_fg(c: avt::Color) -> String {
    match c {
        avt::Color::RGB(_) => {
            // TBD
            "".to_string()
        }
        avt::Color::Indexed(u8) => {
            format!("\x1b[38;5;{}m", u8)
        }
    }
}


fn style_bg(c: avt::Color) -> String {
    match c {
        avt::Color::RGB(_) => {
            // TBD
            "".to_string()
        }
        avt::Color::Indexed(u8) => {
            format!("\x1b[48;5;{}m", u8)
        }
    }
}


pub fn chars_from_lines(lines: &Vec<avt::Line>) -> Array2<u32> {
    let rows = lines.len();
    let line0 = lines.get(0).unwrap();
    let cols = line0.len();

    let v: Vec<_> = lines.iter()
        .flat_map(|l|l.chars().map(|c| u32::from(c)))
        .collect();

    Array2::from_shape_vec([rows, cols], v).unwrap()
}


pub fn truecolor_from_lines<F>(lines: &Vec<avt::Line>, f: F) -> (Array3<u8>, Array2<bool>)
where
    F: Fn(&avt::Pen) -> Option<avt::Color>,
{
    let rows = lines.len();
    let line0 = lines.get(0).unwrap();
    let cols = line0.len();

    let cells = lines.iter().flat_map(|l|l.cells());
    let colors = cells.map(|c| f(c.pen()).map(truecolor_from_avt));
    let vcolors: Vec<_> = colors.collect();
 
    let r = vcolors.iter().map(|c| c.as_ref().map(|cv| cv.r).unwrap_or(0));
    let g = vcolors.iter().map(|c| c.as_ref().map(|cv| cv.g).unwrap_or(0));
    let b = vcolors.iter().map(|c| c.as_ref().map(|cv| cv.b).unwrap_or(0));
    let vm: Vec<_> = r.chain(g).chain(b).collect();
    let vmm: Vec<_> = vcolors.iter().map(|c| c.is_none()).collect();

    let m = Array3::from_shape_vec([3, rows, cols], vm).unwrap();
    let mm = Array2::from_shape_vec([rows, cols], vmm).unwrap();

    (m, mm)
}

pub fn indexedcolor_from_lines<F>(lines: &Vec<avt::Line>, f: F) -> (Array2<u8>, Array2<bool>)
where
    F: Fn(&avt::Pen) -> Option<avt::Color>,
{
    let rows = lines.len();
    let line0 = lines.get(0).unwrap();
    let cols = line0.len();

    let cells = lines.iter().flat_map(|l|l.cells());
    let colors = cells.map(|c| f(c.pen()).map(indexedcolor_from_avt));
    let vcolors: Vec<_> = colors.collect();

    let vm: Vec<_> = vcolors.iter().map(|c| c.unwrap_or(0)).collect();
    let vmm: Vec<_> = vcolors.iter().map(|c| c.is_none()).collect();

    let m = Array2::from_shape_vec([rows, cols], vm).unwrap();
    let mm = Array2::from_shape_vec([rows, cols], vmm).unwrap();

    (m, mm)
}


pub fn render_lines(lines: &Vec<avt::Line>) -> String {
    let mut s = "".to_string();
    for l in lines.iter() {
        let mut foreground: Option<avt::Color> = None;
        let mut background: Option<avt::Color> = None;
        for c in l.cells() {
            let &p = c.pen();
            if p.foreground() != foreground {
                let cc = p
                    .foreground()
                    .map(style_fg)
                    .unwrap_or("\x1b[39m".to_string());
                s.push_str(&cc);
                foreground = p.foreground();
            }
            if p.background() != background {
                let cc = p
                    .background()
                    .map(style_bg)
                    .unwrap_or("\x1b[49m".to_string());
                s.push_str(&cc);
                background = p.background();
            }
            s.push_str(&c.char().to_string());
        }
        s.push_str("\x1b[0m");
        s.push_str("\n")
    }
    s
}
