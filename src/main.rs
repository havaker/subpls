mod user;
use user::*;

fn main() {
    let user = User::login("", "", "en");
    println!("{:?}", user);
    let user = user.unwrap();
    let subs = user.search(vec!["test.mp4".to_owned()]);
    println!("{:?}", subs);
    let result = user.download(subs);
    println!("{:?}", result);
}
