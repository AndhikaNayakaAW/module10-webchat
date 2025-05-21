use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
}

#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    current_user: String,
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        // register with server
        let register_msg = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.clone()),
            data_array: None,
        };
        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&register_msg).unwrap())
        {
            log::debug!("registered user {}", username);
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
            current_user: username,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(raw) => {
                let msg: WebSocketMessage = serde_json::from_str(&raw).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let list = msg.data_array.unwrap_or_default();
                        self.users = list
                            .into_iter()
                            .map(|u| UserProfile {
                                name: u.clone(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                ),
                            })
                            .collect();
                        true
                    }
                    MsgTypes::Message => {
                        if let Some(data) = msg.data {
                            let md: MessageData = serde_json::from_str(&data).unwrap();
                            self.messages.push(md);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            Msg::SubmitMessage => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let val = input.value();
                    if !val.trim().is_empty() {
                        let outgoing = WebSocketMessage {
                            message_type: MsgTypes::Message,
                            data: Some(val.clone()),
                            data_array: None,
                        };
                        if let Err(e) =
                            self.wss.tx.clone().try_send(serde_json::to_string(&outgoing).unwrap())
                        {
                            log::error!("send error: {:?}", e);
                        }
                        input.set_value("");
                    }
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let send_cb = ctx.link().callback(|_| Msg::SubmitMessage);

        html! {
            <div class="flex w-screen h-screen">
                // — Sidebar: user list
                <div class="flex-none w-64 bg-purple-50 p-4 overflow-auto">
                    <h2 class="text-2xl font-bold text-purple-800 mb-4">{"👥 Users"}</h2>
                    { for self.users.iter().map(|u| html!{
                        <div class="flex items-center mb-3 bg-purple-100 rounded-lg p-2 hover:bg-purple-200 transition">
                            <img class="w-10 h-10 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                            <span class="ml-3 text-purple-700">{ &u.name }</span>
                        </div>
                    }) }
                </div>

                // — Main chat area
                <div class="flex-grow flex flex-col bg-white">
                    // Header
                    <header class="h-16 flex items-center bg-purple-200 px-6">
                        <h1 class="text-xl font-semibold text-purple-900">{"🎨 Creative Chat"}</h1>
                    </header>

                    // Creativity banner
                    <div class="px-6 py-2 bg-purple-50 text-purple-800 text-center">
                        {"💡 Stay creative: share your imagination!"}
                    </div>

                    // Messages
                    <div class="flex-grow overflow-auto p-6 space-y-4">
                        { for self.messages.iter().map(|m| {
                            if m.from == self.current_user {
                                html!{
                                    <div class="flex justify-end">
                                        <div class="max-w-md px-4 py-2 bg-green-200 rounded-tl-lg rounded-tr-lg rounded-bl-lg text-gray-800">
                                            { m.message.clone() }
                                        </div>
                                    </div>
                                }
                            } else {
                                let avatar = self
                                    .users
                                    .iter()
                                    .find(|u| u.name == m.from)
                                    .map(|u| u.avatar.clone())
                                    .unwrap_or_default();
                                html!{
                                    <div class="flex items-start">
                                        <img class="w-8 h-8 rounded-full mr-3" src={avatar} alt="avatar"/>
                                        <div class="max-w-md px-4 py-2 bg-purple-200 rounded-tr-lg rounded-br-lg rounded-bl-lg text-gray-800">
                                            <strong>{ format!("{}: ", m.from) }</strong>
                                            { m.message.clone() }
                                        </div>
                                    </div>
                                }
                            }
                        }) }
                    </div>

                    // Input bar
                    <footer class="h-16 flex items-center px-6 bg-purple-50">
                        <input
                            ref={self.chat_input.clone()}
                            type="text"
                            placeholder="Type a message..."
                            class="flex-grow px-4 py-2 bg-white border border-purple-200 rounded-full outline-none focus:border-purple-400"
                        />
                        <button
                            onclick={send_cb}
                            class="ml-4 p-3 bg-indigo-600 rounded-full hover:bg-indigo-700 transition shadow-lg"
                        >
                            <svg class="w-6 h-6 fill-white" viewBox="0 0 24 24">
                                <path d="M0 0h24v24H0z" fill="none"/>
                                <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"/>
                            </svg>
                        </button>
                    </footer>
                </div>
            </div>
        }
    }
}