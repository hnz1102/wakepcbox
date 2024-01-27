#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MenuType {
    MainMenu,
    SubMenu,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum InputTypeChar {
    StringType,
    NumberType,
    HWAddressType,
    TimezoneType,
    ActionType,
    SelectType,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MenuInfo {
    title: String,
    key: String,
    current_value: String,
    set_value: String,
    menu_type: MenuType,
    value_type: InputTypeChar,
    cursor: usize,
    action_flag: bool,
    select_item: Vec<String>,
    current_select_item: usize,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    title: String,
    menu: Vec<MenuInfo>,
    current_select: usize,
}

#[derive(Debug, Clone)]
pub struct MenuTable {
    menu_item: Vec<MenuItem>,
    current_level: usize,
    current_select: usize,
    commit_flag: bool,
    confirming_flag: bool,
    confirmed_flag: bool,
}

#[allow(dead_code)]
impl MenuTable {
    pub fn new() -> Self {
        MenuTable {
            menu_item: Vec::new(),
            current_level: 0,
            current_select: 0,
            commit_flag: false,
            confirming_flag: false,
            confirmed_flag: false,
        }
    }

    pub fn add_menu_item(&mut self, title: &str) {
        self.menu_item.push(MenuItem {
            title: title.to_string(),
            menu: Vec::new(),
            current_select: 0,
        });
    }

    pub fn add_menu(&mut self,  sel: usize,
                                title: &str,
                                key: &str,
                                menu_type: MenuType,
                                value: &str,
                                value_type: InputTypeChar,
                                select_item: Vec<String>) {
        self.menu_item[sel].menu.push(MenuInfo {
            title: title.to_string(),
            key: key.to_string(),
            current_value: value.to_string(),
            set_value: value.to_string(),   
            menu_type,
            cursor: 0,
            value_type: value_type,
            action_flag: false,
            select_item: select_item,
            current_select_item: 0,
        });
    }

    pub fn reset_menu(&mut self) {
        self.current_level = 0;
        self.current_select = 0;
        self.commit_flag = false;
        self.confirming_flag = false;
        self.confirmed_flag = false;
    }

    pub fn get_menu_item_list(&self) -> Vec<MenuItem> {
        self.menu_item.clone()
    }

    pub fn get_menu_list(&self, sel: usize) -> Vec<MenuInfo> {
        self.menu_item[sel].menu.clone()
    }

    pub fn get_menu_item_title(&self, sel: usize) -> String {
        self.menu_item[sel].title.clone()
    }

    pub fn get_menu_title(&self, sel: usize, sel_sub: usize) -> String {
        self.menu_item[sel].menu[sel_sub].title.clone()
    }

    pub fn get_menu_key(&self, sel: usize, sel_sub: usize) -> String {
        self.menu_item[sel].menu[sel_sub].key.clone()
    }

    pub fn get_value(&self, sel: usize, sel_sub: usize) -> String {
        self.menu_item[sel].menu[sel_sub].current_value.clone()
    }

    pub fn set_value(&mut self, sel: usize, sel_sub: usize, value: &str) {
        self.menu_item[sel].menu[sel_sub].current_value = value.to_string();
    }

    pub fn commit_value(&mut self, sel: usize, sel_sub: usize) {
        self.menu_item[sel].menu[sel_sub].set_value = self.menu_item[sel].menu[sel_sub].current_value.clone();
        self.commit_flag = true;
    }

    pub fn get_commit_flag(&self) -> bool {
        self.commit_flag
    }

    pub fn cancel_value(&mut self, sel: usize, sel_sub: usize) {
        self.menu_item[sel].menu[sel_sub].current_value = self.menu_item[sel].menu[sel_sub].set_value.clone();
    }

    pub fn get_current_level(&self) -> usize {
        self.current_level
    }

    pub fn get_current_select(&self) -> usize {
        self.current_select
    }

    pub fn set_current_select(&mut self, sel: usize) {
        self.current_select = sel;
    }

    pub fn set_current_level(&mut self, level: usize) {
        self.current_level = level;
    }

    pub fn get_current_select_sub(&self, sel: usize) -> usize {
        self.menu_item[sel].current_select
    }

    pub fn set_current_select_sub(&mut self, sel: usize, sel_sub: usize) {
        self.menu_item[sel].current_select = sel_sub;
    }

    pub fn get_current_cursor(&self, sel: usize, sel_sub: usize) -> usize {
        self.menu_item[sel].menu[sel_sub].cursor
    }

    pub fn set_current_cursor(&mut self, sel: usize, sel_sub: usize, cursor: usize) {
        self.menu_item[sel].menu[sel_sub].cursor = cursor;
    }

    pub fn get_value_type(&self, sel: usize, sel_sub: usize) -> InputTypeChar {
        self.menu_item[sel].menu[sel_sub].value_type
    }

    pub fn get_action_flag(&self, sel: usize, sel_sub: usize) -> bool {
        self.menu_item[sel].menu[sel_sub].action_flag
    }

    pub fn set_action_flag(&mut self, sel: usize, sel_sub: usize, flag: bool) {
        self.menu_item[sel].menu[sel_sub].action_flag = flag;
    }

    pub fn get_confirming_flag(&self) -> bool {
        self.confirming_flag
    }

    pub fn set_confirming_flag(&mut self, flag: bool) {
        self.confirming_flag = flag;
    }

    pub fn get_confirmed_flag(&self) -> bool {
        self.confirmed_flag
    }

    pub fn set_confirmed_flag(&mut self, flag: bool) {
        self.confirmed_flag = flag;
    }

    pub fn get_select_item(&self, sel: usize, sel_sub: usize, item: usize) -> String {
        self.menu_item[sel].menu[sel_sub].select_item.get(item).unwrap().clone()
    }

    pub fn set_select_item(&mut self, sel: usize, sel_sub: usize, item: Vec<String>) {
        self.menu_item[sel].menu[sel_sub].select_item = item;
    }

    // get index from select item list
    pub fn get_select_item_index(&self, sel: usize, sel_sub: usize, item: &str) -> usize {
        self.menu_item[sel].menu[sel_sub].select_item.iter().position(|x| *x == item.to_string()).unwrap()
    }

    pub fn get_current_select_item(&self, sel: usize, sel_sub: usize) -> usize {
        self.menu_item[sel].menu[sel_sub].current_select_item
    }

    pub fn set_current_select_item(&mut self, sel: usize, sel_sub: usize, item: usize) {
        self.menu_item[sel].menu[sel_sub].current_select_item = item;
    }

    pub fn get_select_item_count(&self, sel: usize, sel_sub: usize) -> usize {
        self.menu_item[sel].menu[sel_sub].select_item.len()
    }

    // get set values from menu table
    pub fn get_all_values(&self) -> Vec<(String, String)> {
        let mut key_values: Vec<(String, String)> = Vec::new();
        for item in &self.menu_item {
            for menu in &item.menu {
                if menu.value_type != InputTypeChar::ActionType || menu.action_flag {
                    key_values.push((menu.key.clone() , menu.current_value.clone()));
                }
            }
        }
        key_values
    }

    // character increment/decrement
    pub fn inc_dec_char(&self, incdec : bool, ch: char, input_type: InputTypeChar) -> char {
        match input_type {
            InputTypeChar::StringType => {
                if incdec {
                    if ch == '~' {
                        return ' ';
                    }
                    else {
                        return (ch as u8 + 1) as char;
                    }
                }
                else {
                    if ch == ' ' {
                        return '~';
                    }
                    else {
                        return (ch as u8 - 1) as char;
                    }
                }
            },
            InputTypeChar::NumberType => {
                if incdec {
                    if ch == '9' {
                        return ' ';
                    }
                    else if ch == ' ' {
                        return '0';
                    }
                    else {
                        return (ch as u8 + 1) as char;
                    }
                }
                else {
                    if ch == '0' {
                        return ' ';
                    }
                    else if ch == ' ' {
                        return '9';
                    }
                    else {
                        return (ch as u8 - 1) as char;
                    }
                }
            },
            InputTypeChar::HWAddressType => {
                if ch == ':' {
                    return ch;
                }
                if incdec {
                    match ch {
                        '0'..='9' => {
                            if ch == '9' {
                                return 'A';
                            }
                            else {
                                return (ch as u8 + 1) as char;
                            }
                        },
                        'A'..='F' => {
                            if ch == 'F' {
                                return '0';
                            }
                            else {
                                return (ch as u8 + 1) as char;
                            }
                        },
                        'a'..='f' => {
                            if ch == 'f' {
                                return '0';
                            }
                            else {
                                return (ch as u8 + 1) as char;
                            }
                        },
                        _ => {
                            return ch;
                        },
                    }
                }
                else {
                    match ch {
                        '0'..='9' => {
                            if ch == '0' {
                                return 'F';
                            }
                            else {
                                return (ch as u8 - 1) as char;
                            }
                        },
                        'A'..='F' => {
                            if ch == 'A' {
                                return '9';
                            }
                            else {
                                return (ch as u8 - 1) as char;
                            }
                        },
                        'a'..='f' => {
                            if ch == 'a' {
                                return '9';
                            }
                            else {
                                return (ch as u8 - 1) as char;
                            }
                        },
                        _ => {
                            return ch;
                        },
                    }
                }
            },
            InputTypeChar::TimezoneType => {
                if incdec {
                    match ch {
                        '0'..='9' => {
                            if ch == '9' {
                                return '0';
                            }
                            else {
                                return (ch as u8 + 1) as char;
                            }
                        },
                        '-' => { return '+'; },
                        '+' => { return '-'; },
                        _ => {
                            return ch;
                        },
                    }
                }
                else {
                    match ch {
                        '0'..='9' => {
                            if ch == '0' {
                                return '9';
                            }
                            else {
                                return (ch as u8 - 1) as char;
                            }
                        },
                        '-' => { return '+'; },
                        '+' => { return '-'; },
                        _ => {
                            return ch;
                        },
                    }
                }
            },
            InputTypeChar::ActionType => {
                return ch;
            },
            InputTypeChar::SelectType => {
                return ch;
            },
        }
    }
}