use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table, Timestamp, ViewContext, rand::Rng};

const GAME_MIN_PLAYER: usize = 2;
const GAME_MAX_PLAYER: usize = 2;
const GAME_CARD_START: usize = 5;
const GAME_ROUND_MAX: usize = GAME_CARD_START - 1;

#[derive(Copy, Clone, PartialEq, SpacetimeType)]
enum Card {
    Secure,
    Bomb,
    Defuse,
}

fn shuffle<T>(vec: &mut Vec<T>, rng: &mut impl Rng) {
    let len = vec.len();
    for i in (1..len).rev() {
        let j = rng.gen_range(0..=i);
        vec.swap(i, j);
    }
}

type PlayerId = u32;

#[spacetimedb::table(accessor = user)]
pub struct User {
    #[primary_key]
    #[auto_inc]
    id: PlayerId, // TODO: convert to UUID
    #[unique]
    identity: Identity,
    name: String,
}

#[spacetimedb::table(accessor = game_lobby)]
pub struct GameLobby {
    #[primary_key]
    #[auto_inc]
    id: u32, // TODO: convert to UUID
    // createdAt: Timestamp, // TODO
    user_ids: Vec<u32>, // Array[Option<Uuid>; 8]?, Head is creator (allowed to start game)
}

#[spacetimedb::table(accessor = game_live)]
pub struct GameLive {
    #[primary_key]
    #[auto_inc]
    id: u32, // TODO: convert to UUID
    // startedAt: Timestamp, // TODO
    user_ids: Vec<u32>,            // Array[Option<Uuid>; 8]?
    players_sherlock: Vec<bool>,   // Array[Option<bool>; 8]?
    players_cards: Vec<Vec<Card>>, // Array[Option<Vec<u8>>; 8]?
    players_call_bomb: Vec<bool>,  // Array[Option<bool>; 8]?
    players_call_defuse: Vec<u8>,  // Array[Option<u8>; 8]?
    players_call_mood: Vec<u8>,    // Array[Option<u8>; 8]?
    timeline_users_idx: Vec<u8>,   // Tail is next player
    timeline_cards: Vec<Card>,     // Index n is picked by timeline_users[n] in timeline_users[n+1]
}

#[spacetimedb::table(accessor = game_done)]
pub struct GameDone {
    #[primary_key]
    #[auto_inc]
    id: u32, // TODO: convert to UUID
    finished_at: Timestamp,
    user_names: Vec<String>, // Array[Option<String>; 8]?
    players_sherlock: Vec<bool>,
    timeline_users_idx: Vec<u8>, // Tail is next player
    timeline_cards: Vec<Card>,   // Index n is picked by timeline_users[n] in timeline_users[n+1]
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    // ctx.db.game.clear() // TODO: remove
    let user = ctx.db.user().identity().find(ctx.sender());
    if user.is_none() {
        ctx.db.user().insert(User {
            id: 0,
            identity: ctx.sender(),
            name: "me".to_string(), // TODO: random name
        });
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(_ctx: &ReducerContext) {
    // Called everytime a client disconnects
}

#[spacetimedb::reducer]
pub fn user_update(ctx: &ReducerContext, name: String) {
    let mut user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    user.name = name;
    ctx.db.user().id().update(user);
}

#[spacetimedb::view(accessor = me, public)]
fn user_me(ctx: &ViewContext) -> Option<User> {
    ctx.db.user().identity().find(ctx.sender())
}

#[spacetimedb::reducer]
pub fn game_create(ctx: &ReducerContext) {
    let creator = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("Creator not found");
    // TODO: Allow only one game per creator
    ctx.db.game_lobby().insert(GameLobby {
        id: 0,
        user_ids: vec![creator.id],
    });
}

#[spacetimedb::reducer]
pub fn game_leave(ctx: &ReducerContext, id: u32) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let mut game = ctx.db.game_lobby().id().find(id).expect("Game not found");
    game.user_ids.retain(|&player_id| player_id != user.id);
    if game.user_ids.is_empty() {
        ctx.db.game_lobby().id().delete(id);
        return;
    }
    ctx.db.game_lobby().id().update(game);
}

#[spacetimedb::reducer]
pub fn game_join(ctx: &ReducerContext, id: u32) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let mut game = ctx.db.game_lobby().id().find(id).expect("Game not found");
    if game.user_ids.len() >= GAME_MAX_PLAYER as usize {
        // TODO: error
        return;
    }
    if !game.user_ids.contains(&user.id) {
        game.user_ids.push(user.id);
    }
    ctx.db.game_lobby().id().update(game);
}

#[spacetimedb::reducer]
pub fn game_start(ctx: &ReducerContext, id: u32) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let game = ctx.db.game_lobby().id().find(id).expect("Game not found");
    if game.user_ids.first() != Some(&user.id) {
        // TODO: error
        return;
    }
    let player_count = game.user_ids.len();
    if player_count < GAME_MIN_PLAYER {
        // TODO: error
        return;
    }
    if player_count > GAME_MAX_PLAYER {
        // TODO: error
        return;
    }

    let starting_player_idx = ctx.rng().gen_range(0..player_count);

    let nb_of_sherlock = (player_count * 2 / 3) as u32;
    let mut players_sherlock: Vec<bool> = game
        .user_ids
        .iter()
        .enumerate()
        .map(|(_, i)| i < &nb_of_sherlock)
        .collect();
    shuffle(&mut players_sherlock, &mut ctx.rng());

    let mut cards: Vec<Card> = Vec::with_capacity(player_count * GAME_CARD_START);
    cards.push(Card::Bomb);
    cards.extend(std::iter::repeat(Card::Defuse).take(player_count));
    cards.extend(
        std::iter::repeat(Card::Secure).take(player_count * GAME_CARD_START - player_count - 1),
    );
    shuffle(&mut cards, &mut ctx.rng());
    let players_cards = distribute_cards(cards, player_count);

    let players_call_bomb = vec![false; player_count];

    let players_call_defuse = vec![0; player_count];

    let players_call_mood = vec![0; player_count];

    ctx.db.game_lobby().id().delete(game.id);
    ctx.db.game_live().insert(GameLive {
        id: 0,
        user_ids: game.user_ids,
        players_sherlock,
        players_cards,
        players_call_bomb,
        players_call_defuse,
        players_call_mood,
        timeline_users_idx: vec![starting_player_idx as u8],
        timeline_cards: vec![],
    });
}

#[spacetimedb::reducer]
pub fn game_pick_card(ctx: &ReducerContext, id: u32, pick_player_idx: u8, card_idx: u8) {
    let pick_player_idx = pick_player_idx as usize;
    let card_idx = card_idx as usize;

    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let mut game = ctx.db.game_live().id().find(id).expect("Game not found");

    let current_player_index = game
        .user_ids
        .iter()
        .position(|&pid| pid == user.id)
        .expect("Current player not found");
    if pick_player_idx == current_player_index {
        panic!("Cannot pick your own card"); // TODO: error
    }
    if game.timeline_users_idx[game.timeline_users_idx.len() - 1] as usize != current_player_index {
        panic!("Not your turn"); // TODO: error
    }
    if pick_player_idx >= game.user_ids.len() {
        panic!("Invalid player index"); // TODO: error
    }

    let card = game.players_cards[pick_player_idx].remove(card_idx); // TODO: graceful error?
    game.timeline_users_idx.push(pick_player_idx as u8);
    game.timeline_cards.push(card);

    if card == Card::Bomb {
        process_game_finished(ctx, &game);
        return;
    }

    let player_count = game.user_ids.len();

    if card == Card::Defuse {
        let nb_of_defuse = game
            .timeline_cards
            .iter()
            .filter(|&&c| c == Card::Defuse)
            .count();
        if nb_of_defuse >= player_count {
            process_game_finished(ctx, &game);
            return;
        }
    }

    if game.timeline_cards.len() >= player_count * GAME_ROUND_MAX {
        process_game_finished(ctx, &game);
        return;
    }

    if game.timeline_cards.len() % player_count == 0 {
        let mut cards = game
            .players_cards
            .iter()
            .flat_map(|c| c.clone())
            .collect::<Vec<_>>();
        shuffle(&mut cards, &mut ctx.rng());
        game.players_cards = distribute_cards(cards, player_count);
    }

    ctx.db.game_live().id().update(game);
}

fn distribute_cards(cards: Vec<Card>, nb_of_players: usize) -> Vec<Vec<Card>> {
    if cards.len() % nb_of_players != 0 {
        panic!("Not enough players");
    }
    let cards_per_player = cards.len() / nb_of_players;
    let mut cards = cards;
    (0..nb_of_players)
        .map(|_| cards.split_off(cards.len() - cards_per_player))
        .collect()
}

fn process_game_finished(ctx: &ReducerContext, game: &GameLive) {
    ctx.db.game_live().id().delete(game.id);
    ctx.db.game_done().insert(GameDone {
        id: 0,
        finished_at: ctx.timestamp,
        user_names: game.user_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(), // TODO: Use actual user names instead of IDs
        players_sherlock: game.players_sherlock.clone(),
        timeline_users_idx: game.timeline_users_idx.clone(),
        timeline_cards: game.timeline_cards.clone(),
    });
}
