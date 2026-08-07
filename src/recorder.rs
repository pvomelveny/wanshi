// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

#[derive(Debug, PartialEq)]
pub enum State {
    LocalLink,
}

impl State {
    pub const fn strify(&self) -> &'static str {
        match self {
            State::LocalLink => "local", // style class name
        }
    }
}
