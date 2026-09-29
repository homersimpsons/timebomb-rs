use core::panic;
use std::iter::{repeat_n, zip};

use spacetimedb::{
    Identity, ReducerContext, SpacetimeType, Table, Timestamp, ViewContext, rand::Rng,
};

const GAME_MIN_PLAYER: usize = 2; // (inclusive)
const GAME_MAX_PLAYER: usize = 8; // (inclusive)
const GAME_CARD_START: usize = 5;
const GAME_ROUND_MAX: usize = GAME_CARD_START - 1;

#[derive(Copy, Clone, PartialEq, SpacetimeType)]
enum Role {
    Sherlock0,
    Sherlock1,
    Sherlock2,
    Sherlock3,
    Sherlock4,
    Moriarty0,
    Moriarty1,
    Moriarty2,
}

const ROLES: [Role; 8] = [
    Role::Sherlock0,
    Role::Moriarty0,
    Role::Sherlock1,
    Role::Moriarty1,
    Role::Sherlock2,
    Role::Sherlock3,
    Role::Moriarty2,
    Role::Sherlock4,
];

#[derive(Copy, Clone, PartialEq, SpacetimeType)]
enum Card {
    Secure,
    Bomb,
    Defuse,
}

#[derive(Copy, Clone, PartialEq, SpacetimeType)]
enum Mood {
    Angel,
    Suspicious,
    Happy,
    Sleepy,
    Devil,
    Detective,
    Cool,
    Laughing,
}

fn shuffle<T>(vec: &mut [T], rng: &mut impl Rng) {
    let len = vec.len();
    for i in (1..len).rev() {
        let j = rng.gen_range(0..=i);
        vec.swap(i, j);
    }
}

type PlayerId = u32; // TODO: convert to UUID
#[spacetimedb::table(accessor = user)]
pub struct User {
    #[primary_key]
    #[auto_inc]
    id: PlayerId,
    #[unique]
    identity: Identity,
    name: String,
}

type GameId = u32; // TODO: convert to UUID
#[spacetimedb::table(accessor = game)]
pub struct Game {
    #[primary_key]
    #[auto_inc]
    id: GameId,
    // TODO? Track lifecycle timestamps
    /// Track which player picked at which turn. Tail is next player to pick.
    timeline_users_id: Vec<PlayerId>,
    /// Track which card was picked at each turn. Index n is picked by timeline_users_id[n] in timeline_users_id[n+1].
    timeline_cards: Vec<Card>,
}

#[spacetimedb::table(accessor = game_lobby, public)]
pub struct GameLobby {
    #[primary_key]
    user_id: PlayerId,
    #[index(btree)]
    game_id: GameId,
    ready: bool,
}

#[spacetimedb::table(accessor = game_live)]
pub struct GameLive {
    #[primary_key]
    user_id: PlayerId,
    #[index(btree)]
    game_id: GameId,
    player_role: Role,
    player_cards: Vec<Card>,
    player_call_bomb: bool, // TODO: Add uncalled
    player_call_defuse: u8, // TODO: Add uncalled
    player_call_mood: Option<Mood>,
}

#[derive(SpacetimeType)]
struct PlayerSherlock {
    name: String,
    is_moriarty: bool,
}
#[spacetimedb::table(accessor = game_done, public)]
pub struct GameDone {
    #[primary_key]
    #[auto_inc]
    id: u32, // TODO: convert to UUID
    #[index(btree)]
    finished_at: Timestamp,
    players: Vec<PlayerSherlock>,
    timeline_users_idx: Vec<u8>,
    timeline_cards: Vec<Card>,
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
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
pub fn identity_disconnected(ctx: &ReducerContext) {
    #[expect(clippy::collapsible_if)]
    if let Some(user) = ctx.db.user().identity().find(ctx.sender()) {
        if let Some(mut game) = ctx.db.game_lobby().user_id().find(user.id) {
            game.ready = false;
            ctx.db.game_lobby().user_id().update(game);
        }
        // TODO: Mark absent for game_live
    }
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
pub fn game_create(ctx: &ReducerContext) -> Result<(), String> {
    let creator = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");

    let game = ctx.db.game_lobby().user_id().find(creator.id);
    if game.is_some() {
        return Err("User is already in a game".into());
    }
    let game = ctx.db.game().insert(Game {
        id: 0,
        timeline_users_id: Vec::new(),
        timeline_cards: Vec::new(),
    });
    ctx.db.game_lobby().insert(GameLobby {
        user_id: creator.id,
        game_id: game.id,
        ready: false,
    });

    Ok(())
}

#[spacetimedb::reducer]
pub fn game_leave(ctx: &ReducerContext) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let game = ctx
        .db
        .game_lobby()
        .user_id()
        .find(user.id)
        .expect("Game not found");

    ctx.db.game_lobby().delete(game);
}

#[spacetimedb::reducer]
pub fn game_join(ctx: &ReducerContext, id: GameId) -> Result<(), String> {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let game_lobby = ctx.db.game_lobby().user_id().find(user.id);
    if game_lobby.is_some() {
        return Err("User is already in a lobby".into());
    }

    let game_live = ctx.db.game_live().user_id().find(user.id);
    if game_live.is_some() {
        return Err("User is already in a live game".into());
    }

    let user_count = ctx.db.game_lobby().game_id().filter(id).count();
    if user_count > GAME_MAX_PLAYER {
        return Err("Lobby is full".into());
    }
    if user_count == 0 {
        return Err("Lobby is empty".into());
    }

    ctx.db.game_lobby().insert(GameLobby {
        user_id: user.id,
        game_id: id,
        ready: false,
    });
    Ok(())
}

#[spacetimedb::reducer]
pub fn game_ready(ctx: &ReducerContext, ready: bool) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let mut game_lobby = ctx
        .db
        .game_lobby()
        .user_id()
        .find(user.id)
        .expect("Game not found");
    if game_lobby.ready == ready {
        return;
    }
    game_lobby.ready = ready;
    if !game_lobby.ready {
        ctx.db.game_lobby().user_id().update(game_lobby);
        return;
    }
    let game_id = game_lobby.game_id;
    ctx.db.game_lobby().user_id().update(game_lobby);
    let game_users = ctx
        .db
        .game_lobby()
        .game_id()
        .filter(game_id)
        .collect::<Vec<_>>();
    let player_count = game_users.len();
    if player_count < GAME_MIN_PLAYER {
        return;
    }
    assert!(
        player_count <= GAME_MAX_PLAYER,
        "Player count exceeds maximum allowed"
    );
    if !game_users.iter().all(|g| g.ready) {
        return; // Wait for every one to be ready to start the game
    }

    let mut game = ctx.db.game().id().find(game_id).expect("Game not found");

    let user_ids = game_users.iter().map(|g| g.user_id).collect::<Vec<_>>();

    let starting_player_idx = ctx.rng().gen_range(0..user_ids.len());
    let starting_player_id = user_ids[starting_player_idx];

    // TODO: Follow rules regarding roles assignations
    let mut players_roles = ROLES.iter().take(player_count).cloned().collect::<Vec<_>>();
    shuffle(&mut players_roles, &mut ctx.rng());

    let mut cards: Vec<Card> = Vec::with_capacity(player_count * GAME_CARD_START);
    cards.push(Card::Bomb);
    cards.extend(repeat_n(Card::Defuse, player_count));
    cards.extend(repeat_n(
        Card::Secure,
        player_count * GAME_CARD_START - player_count - 1,
    ));
    shuffle(&mut cards, &mut ctx.rng());
    let players_cards = distribute_cards(cards, player_count);

    ctx.db.game_lobby().game_id().delete(game_id);
    for row in zip(user_ids, zip(players_roles, players_cards)) {
        ctx.db.game_live().insert(GameLive {
            user_id: row.0,
            game_id: game.id,
            player_role: row.1.0,
            player_cards: row.1.1,
            player_call_bomb: false,
            player_call_defuse: 0,
            player_call_mood: None,
        });
    }

    game.timeline_users_id.push(starting_player_id);
    ctx.db.game().id().update(game);
}

#[derive(SpacetimeType)]
pub struct MyGameLive {
    player_rows: Vec<GameLive>,
    timeline_users_id: Vec<PlayerId>,
    timeline_cards: Vec<Card>,
}
#[spacetimedb::view(accessor = my_game_live, public)]
pub fn my_game_live(ctx: &ViewContext) -> Option<MyGameLive> {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let player_game = ctx.db.game_live().user_id().find(user.id)?;
    let game_players = ctx
        .db
        .game_live()
        .game_id()
        .filter(player_game.game_id)
        .collect::<Vec<_>>();
    let game = ctx
        .db
        .game()
        .id()
        .find(player_game.game_id)
        .expect("Game not found");

    // TODO: Redact other players information
    Some(MyGameLive {
        player_rows: game_players,
        timeline_users_id: game.timeline_users_id.clone(),
        timeline_cards: game.timeline_cards.clone(),
    })
}

#[spacetimedb::reducer]
pub fn game_pick_card(ctx: &ReducerContext, pick_player_id: u32, card_idx: u8) {
    let card_idx = card_idx as usize;

    let (user, game_picker) = relove_user_game_live(ctx);
    if pick_player_id == user.id {
        panic!("Cannot pick your own card"); // TODO: error
    }
    let mut game = ctx
        .db
        .game()
        .id()
        .find(game_picker.game_id)
        .expect("Game not found");
    if game.timeline_users_id[game.timeline_users_id.len() - 1] != user.id {
        panic!("Not your turn"); // TODO: error
    }

    let mut game_players = ctx
        .db
        .game_live()
        .game_id()
        .filter(game_picker.game_id)
        .collect::<Vec<_>>();

    let game_picked = game_players
        .iter_mut()
        .find(|gp| gp.user_id == pick_player_id)
        .expect("Players are not in the same game");
    let card = game_picked.player_cards.remove(card_idx); // TODO: graceful error?
    game.timeline_users_id.push(game_picked.user_id);
    game.timeline_cards.push(card);

    if card == Card::Bomb {
        process_game_finished(ctx, &game, &game_players);
        return;
    }

    let player_count = game_players.len();

    if card == Card::Defuse {
        let nb_of_defuse = game
            .timeline_cards
            .iter()
            .filter(|&&c| c == Card::Defuse)
            .count();
        if nb_of_defuse >= player_count {
            process_game_finished(ctx, &game, &game_players);
            return;
        }
    }

    if game.timeline_cards.len() >= player_count * GAME_ROUND_MAX {
        process_game_finished(ctx, &game, &game_players);
        return;
    }

    game_players.iter_mut().for_each(|gp| {
        gp.player_call_bomb = false;
        gp.player_call_defuse = 0;
        gp.player_call_mood = None;
    });

    if game.timeline_cards.len().is_multiple_of(player_count) {
        let mut cards = game_players
            .iter()
            .flat_map(|gp| gp.player_cards.clone())
            .collect::<Vec<_>>();
        shuffle(&mut *cards.as_mut_slice(), &mut ctx.rng());
        let distributed_cards = distribute_cards(cards, player_count);
        game_players
            .iter_mut()
            .zip(distributed_cards)
            .for_each(|(gp, new_cards)| {
                gp.player_cards = new_cards;
            });
    }

    for gp in game_players {
        ctx.db.game_live().user_id().update(gp);
    }
    ctx.db.game().id().update(game);
}

fn distribute_cards(cards: Vec<Card>, nb_of_players: usize) -> Vec<Vec<Card>> {
    assert!(
        cards.len().is_multiple_of(nb_of_players),
        "Not enough players"
    );
    let cards_per_player = cards.len() / nb_of_players;
    let mut cards = cards;
    (0..nb_of_players)
        .map(|_| cards.split_off(cards.len() - cards_per_player))
        .collect()
}

#[spacetimedb::reducer]
fn game_call_mood(ctx: &ReducerContext, mood: Option<Mood>) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.player_call_mood = mood;
    ctx.db.game_live().user_id().update(player_game);
}

#[spacetimedb::reducer]
fn game_call_defuse(ctx: &ReducerContext, defuse: u8) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.player_call_defuse = defuse;
    ctx.db.game_live().user_id().update(player_game);
}

#[spacetimedb::reducer]
fn game_call_bomb(ctx: &ReducerContext, bomb: bool) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.player_call_bomb = bomb;
    ctx.db.game_live().user_id().update(player_game);
}

fn process_game_finished(ctx: &ReducerContext, game: &Game, game_players: &[GameLive]) {
    ctx.db.game_live().game_id().delete(game.id);
    ctx.db.game().id().delete(game.id);

    let mut user_ids = game_players.iter().map(|gp| gp.user_id).collect::<Vec<_>>();
    user_ids.sort();
    let inserted_game = ctx.db.game_done().insert(GameDone {
        id: 0,
        finished_at: ctx.timestamp,
        players: game_players
            .iter()
            .map(|gp| PlayerSherlock {
                name: gp.user_id.to_string(), // TODO: Use actual user names instead of IDs
                is_moriarty: matches!(
                    gp.player_role,
                    Role::Moriarty0 | Role::Moriarty1 | Role::Moriarty2
                ),
            })
            .collect::<Vec<_>>(),
        timeline_users_idx: game
            .timeline_users_id
            .iter()
            .map(|user_id| user_ids.binary_search(user_id).unwrap() as u8)
            .collect::<Vec<_>>(),
        timeline_cards: game.timeline_cards.clone(),
    });
    // TODO: Figure out a better way to manage old finished games
    ctx.db.game_done().id().delete(inserted_game.id - 5);
}

fn relove_user_game_live(ctx: &ReducerContext) -> (User, GameLive) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let player_game = ctx
        .db
        .game_live()
        .user_id()
        .find(user.id)
        .expect("User not in live game");

    (user, player_game)
}
