use photogrammetry_kernel::{features, Image};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut all = Vec::new();
    for path in args {
        let b = std::fs::read(&path).unwrap();
        let mut it = b.splitn(4, |&c| c == b'\n');
        let _ = it.next();
        let dim = std::str::from_utf8(it.next().unwrap()).unwrap();
        let d: Vec<usize> = dim.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let _ = it.next();
        let image = Image {
            width: d[0],
            height: d[1],
            rgb: it.next().unwrap().to_vec(),
            focal: 2266.,
        };
        let f = features::extract(&image, 900).unwrap();
        println!("{path} {}", f.len());
        all.push(f);
    }
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            println!("{i}-{j}: {}", features::matches(&all[i], &all[j]).len());
        }
    }
}
