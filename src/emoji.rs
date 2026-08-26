/// Embedded offline Unicode Emoji Picker with English & Vietnamese keyword search.
use crate::launcher::{LauncherItem, ItemType};

pub struct EmojiItem {
    pub char: &'static str,
    pub name: &'static str,
    pub keywords: &'static str,
}

pub const EMOJIS: &[EmojiItem] = &[
    // Smileys & Emotion
    EmojiItem { char: "😀", name: "Grinning Face", keywords: "smile happy grin cuoi vui mat cuoi" },
    EmojiItem { char: "😃", name: "Grinning Face with Big Eyes", keywords: "smile happy cuoi tuoi" },
    EmojiItem { char: "😄", name: "Grinning Face with Smiling Eyes", keywords: "smile happy hehe cuoi" },
    EmojiItem { char: "😁", name: "Beaming Face with Smiling Eyes", keywords: "grin teeth cuoi toe toet" },
    EmojiItem { char: "😆", name: "Grinning Squinting Face", keywords: "laugh haha lol vui" },
    EmojiItem { char: "😅", name: "Grinning Face with Sweat", keywords: "sweat nervous hot cuoi tru cuoi ngai" },
    EmojiItem { char: "🤣", name: "Rolling on the Floor Laughing", keywords: "rofl lol laugh cuoi bo cuoi lan lon" },
    EmojiItem { char: "😂", name: "Face with Tears of Joy", keywords: "joy tears lol laugh haha cuoi ra nuoc mat" },
    EmojiItem { char: "🙂", name: "Slightly Smiling Face", keywords: "smile cuoi nhe bim" },
    EmojiItem { char: "😉", name: "Winking Face", keywords: "wink nhay mat" },
    EmojiItem { char: "😊", name: "Smiling Face with Smiling Eyes", keywords: "blush happy hanh phuc ngai" },
    EmojiItem { char: "😇", name: "Smiling Face with Halo", keywords: "angel innocent thien than thanh thien" },
    EmojiItem { char: "🥰", name: "Smiling Face with Hearts", keywords: "love adore yeu thuong thich" },
    EmojiItem { char: "😍", name: "Heart Eyes", keywords: "love crsuh me man yeu" },
    EmojiItem { char: "🤩", name: "Star-Struck", keywords: "stars excited toa sang bat ngo" },
    EmojiItem { char: "😘", name: "Face Blowing a Kiss", keywords: "kiss love hon yeu" },
    EmojiItem { char: "😋", name: "Face Savoring Food", keywords: "delicious yum ngon theu" },
    EmojiItem { char: "😛", name: "Face with Tongue", keywords: "tongue le luoi treu" },
    EmojiItem { char: "😜", name: "Winking Face with Tongue", keywords: "wink tongue treu choc" },
    EmojiItem { char: "🤪", name: "Zany Face", keywords: "crazy ngong cuong di" },
    EmojiItem { char: "😝", name: "Squinting Face with Tongue", keywords: "playful le luoi" },
    EmojiItem { char: "🤑", name: "Money-Mouth Face", keywords: "money rich tien giau" },
    EmojiItem { char: "🤗", name: "Smiling Face with Open Hands", keywords: "hug om chao don" },
    EmojiItem { char: "🤫", name: "Shushing Face", keywords: "quiet secret im lang bi mat" },
    EmojiItem { char: "🤔", name: "Thinking Face", keywords: "think suy nghi phan van" },
    EmojiItem { char: "🤐", name: "Zipper-Mouth Face", keywords: "silent khao mieng kin mieng" },
    EmojiItem { char: "🤨", name: "Face with Raised Eyebrow", keywords: "skeptical nghi ngo" },
    EmojiItem { char: "😐", name: "Neutral Face", keywords: "neutral poker binh thuong vo cam" },
    EmojiItem { char: "😑", name: "Expressionless Face", keywords: "blank chang noi nen loi" },
    EmojiItem { char: "😶", name: "Face Without Mouth", keywords: "silent cam lang" },
    EmojiItem { char: "😏", name: "Smirking Face", keywords: "smirk nham hiem cuoi nua mieng" },
    EmojiItem { char: "😒", name: "Unamused Face", keywords: "annoyed chan nan kho chiu" },
    EmojiItem { char: "🙄", name: "Face with Rolling Eyes", keywords: "eye roll dao mat chan" },
    EmojiItem { char: "😬", name: "Grimacing Face", keywords: "grimace ngai ngung suong suong" },
    EmojiItem { char: "🤥", name: "Lying Face", keywords: "lie pinocchio noi doi" },
    EmojiItem { char: "😌", name: "Relieved Face", keywords: "relieved nhe nhom yen tam" },
    EmojiItem { char: "😔", name: "Pensive Face", keywords: "sad u ru buon" },
    EmojiItem { char: "😪", name: "Sleepy Face", keywords: "tired buon ngu met" },
    EmojiItem { char: "🤤", name: "Drooling Face", keywords: "drool chay nuoc dai them" },
    EmojiItem { char: "😴", name: "Sleeping Face", keywords: "sleep zzz ngu" },
    EmojiItem { char: "😷", name: "Face with Medical Mask", keywords: "mask sick khau trang om benh" },
    EmojiItem { char: "🤒", name: "Face with Thermometer", keywords: "fever sick sot om" },
    EmojiItem { char: "🤕", name: "Face with Head-Bandage", keywords: "hurt injured bang dau thuong" },
    EmojiItem { char: "🤢", name: "Nauseated Face", keywords: "vomit sick buon non khong khoe" },
    EmojiItem { char: "🤮", name: "Face Vomiting", keywords: "puke o mua non" },
    EmojiItem { char: "🤧", name: "Sneezing Face", keywords: "sneeze hat hoi cum" },
    EmojiItem { char: "🥵", name: "Hot Face", keywords: "heat summer nong buc" },
    EmojiItem { char: "🥶", name: "Cold Face", keywords: "freezing winter lanh bang" },
    EmojiItem { char: "🥴", name: "Woozy Face", keywords: "drunk say choang vang" },
    EmojiItem { char: "😵", name: "Dizzy Face", keywords: "dizzy hoa mat chong mat" },
    EmojiItem { char: "🤯", name: "Exploding Head", keywords: "mind blown no nao soc bat ngo" },
    EmojiItem { char: "🤠", name: "Cowboy Hat Face", keywords: "cowboy cao boi" },
    EmojiItem { char: "🥳", name: "Partying Face", keywords: "party celebration sinh nhat an mung" },
    EmojiItem { char: "😎", name: "Smiling Face with Sunglasses", keywords: "cool ngau kinh ram sunglasses" },
    EmojiItem { char: "🤓", name: "Nerd Face", keywords: "geek nerd mot sach kinh can" },
    EmojiItem { char: "🧐", name: "Face with Monocle", keywords: "monocle soi xem xet" },
    EmojiItem { char: "😕", name: "Confused Face", keywords: "confused boi roi kho hieu" },
    EmojiItem { char: "😟", name: "Worried Face", keywords: "worried lo lang" },
    EmojiItem { char: "🙁", name: "Slightly Frowning Face", keywords: "sad buon nhe" },
    EmojiItem { char: "😮", name: "Face with Open Mouth", keywords: "surprise a ha ha hoc mom" },
    EmojiItem { char: "😯", name: "Hushed Face", keywords: "surprised nga nhien" },
    EmojiItem { char: "😲", name: "Astonished Face", keywords: "shocked kinh ngac" },
    EmojiItem { char: "😳", name: "Flushed Face", keywords: "blushing do mat ngai" },
    EmojiItem { char: "🥺", name: "Pleading Face", keywords: "beg puppy eyes lam nung xin xin" },
    EmojiItem { char: "😦", name: "Frowning Face with Open Mouth", keywords: "worried hoang hot" },
    EmojiItem { char: "😨", name: "Fearful Face", keywords: "fear so hai" },
    EmojiItem { char: "😰", name: "Anxious Face with Sweat", keywords: "anxious toat mo hoi lo" },
    EmojiItem { char: "😥", name: "Sad but Relieved Face", keywords: "sad nhe nhom" },
    EmojiItem { char: "😢", name: "Crying Face", keywords: "cry tear khoc buon" },
    EmojiItem { char: "😭", name: "Loudly Crying Face", keywords: "sob cry khoc to oa khoc" },
    EmojiItem { char: "😱", name: "Face Screaming in Fear", keywords: "scream shock het len kinh hoang" },
    EmojiItem { char: "😖", name: "Confounded Face", keywords: "suffering dau kho nhan nho" },
    EmojiItem { char: "😣", name: "Persevering Face", keywords: "struggle gang guong" },
    EmojiItem { char: "😞", name: "Disappointed Face", keywords: "sad that vong" },
    EmojiItem { char: "😓", name: "Downcast Face with Sweat", keywords: "sweat vat va" },
    EmojiItem { char: "😩", name: "Weary Face", keywords: "tired met moi than tho" },
    EmojiItem { char: "😫", name: "Tired Face", keywords: "exhausted kiet suc" },
    EmojiItem { char: "🥱", name: "Yawning Face", keywords: "yawn ngap buon ngu" },
    EmojiItem { char: "😤", name: "Face with Steam From Nose", keywords: "angry proud hung ho tuc gian" },
    EmojiItem { char: "😡", name: "Enraged Face", keywords: "angry mad gian du tuc dien" },
    EmojiItem { char: "😠", name: "Angry Face", keywords: "mad tuc gian cau" },
    EmojiItem { char: "🤬", name: "Face with Symbols on Mouth", keywords: "curse swear chui the tuc" },
    EmojiItem { char: "😈", name: "Smiling Face with Horns", keywords: "devil ac quy doc ac" },
    EmojiItem { char: "👿", name: "Angry Face with Horns", keywords: "devil evil quy du" },
    EmojiItem { char: "💀", name: "Skull", keywords: "death dead xau xac dau lau chet" },
    EmojiItem { char: "💩", name: "Pile of Poo", keywords: "poop cut phan hai huoc" },
    EmojiItem { char: "🤡", name: "Clown Face", keywords: "clown he chu he" },
    EmojiItem { char: "👻", name: "Ghost", keywords: "ghost ma quy halloween" },
    EmojiItem { char: "👽", name: "Alien", keywords: "alien ufo nguoi ngoai hanh tinh" },
    EmojiItem { char: "🤖", name: "Robot", keywords: "bot ai robot nguoi may" },

    // Gestures & Body
    EmojiItem { char: "👋", name: "Waving Hand", keywords: "wave hello bye chao tam biet" },
    EmojiItem { char: "🤚", name: "Raised Back of Hand", keywords: "hand ban tay gio tay" },
    EmojiItem { char: "🖐️", name: "Hand with Fingers Splayed", keywords: "hand ban tay 5 ngon" },
    EmojiItem { char: "✋", name: "Raised Hand", keywords: "high five stop dung lai" },
    EmojiItem { char: "🖖", name: "Vulcan Salute", keywords: "spock star trek chao" },
    EmojiItem { char: "👌", name: "OK Hand", keywords: "ok perfect tot dung roi" },
    EmojiItem { char: "🤌", name: "Pinched Fingers", keywords: "italian what lam sao" },
    EmojiItem { char: "🤏", name: "Pinching Hand", keywords: "small little mot chut ty teo" },
    EmojiItem { char: "✌️", name: "Victory Hand", keywords: "peace victory hai ngon chao" },
    EmojiItem { char: "🤞", name: "Crossed Fingers", keywords: "luck hope cau may chuc may man" },
    EmojiItem { char: "🤟", name: "Love-You Gesture", keywords: "ily love yeu ban" },
    EmojiItem { char: "🤘", name: "Sign of the Horns", keywords: "rock metal ngau quet" },
    EmojiItem { char: "🤙", name: "Call Me Hand", keywords: "call phone goi dien" },
    EmojiItem { char: "👈", name: "Backhand Index Pointing Left", keywords: "left chi ben trai" },
    EmojiItem { char: "👉", name: "Backhand Index Pointing Right", keywords: "right chi ben phai" },
    EmojiItem { char: "👆", name: "Backhand Index Pointing Up", keywords: "up chi len tren" },
    EmojiItem { char: "👇", name: "Backhand Index Pointing Down", keywords: "down chi xuong duoi" },
    EmojiItem { char: "☝️", name: "Index Pointing Up", keywords: "up one so mot y kien" },
    EmojiItem { char: "👍", name: "Thumbs Up", keywords: "like approve good ok thich duoc tuyet" },
    EmojiItem { char: "👎", name: "Thumbs Down", keywords: "dislike bad khong thich che" },
    EmojiItem { char: "✊", name: "Raised Fist", keywords: "fist power nam dam quyet tam" },
    EmojiItem { char: "👊", name: "Oncoming Fist", keywords: "punch brofist dam chuc mung" },
    EmojiItem { char: "🤛", name: "Left-Facing Fist", keywords: "fist bump cham tay" },
    EmojiItem { char: "🤜", name: "Right-Facing Fist", keywords: "fist bump cham tay" },
    EmojiItem { char: "👏", name: "Clapping Hands", keywords: "clap applause vo tay chuc mung" },
    EmojiItem { char: "🙌", name: "Raising Hands", keywords: "hooray praise an mung hai tay" },
    EmojiItem { char: "👐", name: "Open Hands", keywords: "open hands mo long" },
    EmojiItem { char: "🤲", name: "Palms Up Together", keywords: "prayer xin nguyen" },
    EmojiItem { char: "🤝", name: "Handshake", keywords: "deal agreement bat tay dong y hop tac" },
    EmojiItem { char: "🙏", name: "Folded Hands", keywords: "pray please thank you cam on xin loi chuc" },
    EmojiItem { char: "✍️", name: "Writing Hand", keywords: "write pen viet chu ky" },
    EmojiItem { char: "💅", name: "Nail Polish", keywords: "nails beauty sang chanh lam mong" },
    EmojiItem { char: "🤳", name: "Selfie", keywords: "camera phone tu suong" },
    EmojiItem { char: "💪", name: "Flexed Biceps", keywords: "muscle strong co bap khoe manh co len" },
    EmojiItem { char: "🧠", name: "Brain", keywords: "brain smart nao thong minh" },
    EmojiItem { char: "👀", name: "Eyes", keywords: "look see doi mat nhin dom" },
    EmojiItem { char: "👁️", name: "Eye", keywords: "eye con mat" },

    // Hearts & Symbols
    EmojiItem { char: "❤️", name: "Red Heart", keywords: "love heart tim yeu thuong do" },
    EmojiItem { char: "🧡", name: "Orange Heart", keywords: "orange heart tim cam" },
    EmojiItem { char: "💛", name: "Yellow Heart", keywords: "yellow heart tim vang" },
    EmojiItem { char: "💚", name: "Green Heart", keywords: "green heart tim xanh la" },
    EmojiItem { char: "💙", name: "Blue Heart", keywords: "blue heart tim xanh duong" },
    EmojiItem { char: "💜", name: "Purple Heart", keywords: "purple heart tim tim" },
    EmojiItem { char: "🖤", name: "Black Heart", keywords: "black heart tim den" },
    EmojiItem { char: "🤍", name: "White Heart", keywords: "white heart tim trang" },
    EmojiItem { char: "🤎", name: "Brown Heart", keywords: "brown heart tim nau" },
    EmojiItem { char: "💔", name: "Broken Heart", keywords: "breakup sad vo tim that tinh" },
    EmojiItem { char: "💖", name: "Sparkling Heart", keywords: "sparkle tim lap lanh" },
    EmojiItem { char: "💗", name: "Growing Heart", keywords: "growing tim lon dan" },
    EmojiItem { char: "💓", name: "Beating Heart", keywords: "heartbeat tim dap rung dong" },
    EmojiItem { char: "💞", name: "Revolving Hearts", keywords: "love tim quay" },
    EmojiItem { char: "💕", name: "Two Hearts", keywords: "love hai trai tim" },
    EmojiItem { char: "💯", name: "Hundred Points", keywords: "100 perfect diem muoi tuyet doi" },
    EmojiItem { char: "💢", name: "Anger Symbol", keywords: "angry tuc gian" },
    EmojiItem { char: "💥", name: "Collision", keywords: "boom explosion no va cham" },
    EmojiItem { char: "💫", name: "Dizzy", keywords: "star hoa mat sao" },
    EmojiItem { char: "💦", name: "Sweat Droplets", keywords: "water giot nuoc mo hoi" },
    EmojiItem { char: "💨", name: "Dashing Away", keywords: "fast wind chay nhanh gio" },
    EmojiItem { char: "🕳️", name: "Hole", keywords: "hole ho sau" },
    EmojiItem { char: "💣", name: "Bomb", keywords: "bomb qua bom" },
    EmojiItem { char: "💬", name: "Speech Balloon", keywords: "chat comment tin nhan bong bong" },
    EmojiItem { char: "🗨️", name: "Left Speech Bubble", keywords: "talk hoi thoai" },
    EmojiItem { char: "💭", name: "Thought Balloon", keywords: "thought suy nghi" },

    // Tech & Tools & Productivity
    EmojiItem { char: "🚀", name: "Rocket", keywords: "rocket launch speed tau vu tru phong nhanh" },
    EmojiItem { char: "💻", name: "Laptop", keywords: "computer macbook laptop may tinh code" },
    EmojiItem { char: "🖥️", name: "Desktop Computer", keywords: "pc monitor may tinh ban" },
    EmojiItem { char: "📱", name: "Mobile Phone", keywords: "iphone android smartphone dien thoai" },
    EmojiItem { char: "💡", name: "Light Bulb", keywords: "idea bulb sang kien bong den" },
    EmojiItem { char: "🔥", name: "Fire", keywords: "fire flame hot lua chay hot trend" },
    EmojiItem { char: "✨", name: "Sparkles", keywords: "magic sparkle star lap lanh dep" },
    EmojiItem { char: "⭐", name: "Star", keywords: "star ngoi sao danh gia" },
    EmojiItem { char: "🌟", name: "Glowing Star", keywords: "shine ngoi sao sang" },
    EmojiItem { char: "🎉", name: "Party Popper", keywords: "tada party celebration tiec phao giay chuc mung" },
    EmojiItem { char: "🎊", name: "Confetti Ball", keywords: "confetti le hoi" },
    EmojiItem { char: "🎁", name: "Wrapped Gift", keywords: "gift present qua tang sinh nhat" },
    EmojiItem { char: "🏆", name: "Trophy", keywords: "winner cup vo dich cup giai thuong" },
    EmojiItem { char: "🥇", name: "1st Place Medal", keywords: "gold medal huy chuong vang hang nhat" },
    EmojiItem { char: "🥈", name: "2nd Place Medal", keywords: "silver huy chuong bac" },
    EmojiItem { char: "🥉", name: "3rd Place Medal", keywords: "bronze huy chuong dong" },
    EmojiItem { char: "✅", name: "Check Mark Button", keywords: "check pass success ok hoan thanh xong dung" },
    EmojiItem { char: "✔️", name: "Check Mark", keywords: "tick check dau tich" },
    EmojiItem { char: "❌", name: "Cross Mark", keywords: "cross x no error fail sai huy bo" },
    EmojiItem { char: "❎", name: "Cross Mark Button", keywords: "cancel bo qua" },
    EmojiItem { char: "⚠️", name: "Warning", keywords: "warning alert canhh bao luu y" },
    EmojiItem { char: "⛔", name: "No Entry", keywords: "forbidden cam vao" },
    EmojiItem { char: "🚫", name: "Prohibited", keywords: "no ban cam" },
    EmojiItem { char: "🔍", name: "Magnifying Glass Left", keywords: "search find tim kiem soi" },
    EmojiItem { char: "🔎", name: "Magnifying Glass Right", keywords: "search tim kiem" },
    EmojiItem { char: "🔒", name: "Locked", keywords: "lock secure khoa bao mat" },
    EmojiItem { char: "🔓", name: "Unlocked", keywords: "unlock mo khoa" },
    EmojiItem { char: "🔑", name: "Key", keywords: "key password chia khoa mat khau" },
    EmojiItem { char: "🔨", name: "Hammer", keywords: "tool build bua sua chua" },
    EmojiItem { char: "⚙️", name: "Gear", keywords: "settings config cai dat banh rang" },
    EmojiItem { char: "🔧", name: "Wrench", keywords: "tool fix co le sua" },
    EmojiItem { char: "📦", name: "Package", keywords: "box package kien hang dong goi" },
    EmojiItem { char: "📁", name: "File Folder", keywords: "folder directory thu muc" },
    EmojiItem { char: "📂", name: "Open File Folder", keywords: "folder mo thu muc" },
    EmojiItem { char: "📄", name: "Page Facing Up", keywords: "file document tai lieu trang van ban" },
    EmojiItem { char: "📊", name: "Bar Chart", keywords: "chart stats bieu do thong ke" },
    EmojiItem { char: "📈", name: "Chart Increasing", keywords: "growth up tang truong" },
    EmojiItem { char: "📉", name: "Chart Decreasing", keywords: "down giam sut" },
    EmojiItem { char: "📌", name: "Pushpin", keywords: "pin ghim dinh" },
    EmojiItem { char: "📍", name: "Round Pushpin", keywords: "location map dia diem vi tri" },
    EmojiItem { char: "🔗", name: "Link", keywords: "link url lien ket" },
    EmojiItem { char: "⏳", name: "Hourglass Not Done", keywords: "time loading dong ho cat cho" },
    EmojiItem { char: "⏰", name: "Alarm Clock", keywords: "clock time bao thuc hen gio" },
    EmojiItem { char: "⏱️", name: "Stopwatch", keywords: "timer bam gio" },
    EmojiItem { char: "📅", name: "Calendar", keywords: "date schedule lich ngay thang" },
];

/// Searches emojis by fuzzy/substring matching in names and keywords.
pub fn search_emojis(query: &str) -> Vec<LauncherItem> {
    let q = query.trim().to_lowercase();
    let is_empty = q.is_empty();

    let mut matched = Vec::new();

    for item in EMOJIS {
        let score = if is_empty {
            1
        } else if item.name.to_lowercase().contains(&q) {
            100
        } else if item.keywords.contains(&q) {
            50
        } else {
            0
        };

        if score > 0 {
            matched.push((item, score));
        }
    }

    matched.sort_by(|a, b| b.1.cmp(&a.1));

    matched
        .into_iter()
        .take(50)
        .map(|(emoji, _)| {
            LauncherItem::new(
                format!("{} {}", emoji.char, emoji.name),
                emoji.char.to_string(),
                ItemType::Calc, // Copies value to clipboard on activate
                Some(format!("Copy \"{}\" to clipboard", emoji.char)),
                false,
                None,
            )
        })
        .collect()
}
