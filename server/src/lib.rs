use core::panic;
use std::{
    iter::{once, repeat, repeat_n, zip},
    time::Duration,
};

use spacetimedb::{
    Identity, ReducerContext, ScheduleAt, SpacetimeType, Table, Timestamp, ViewContext,
    rand::{Rng, seq::SliceRandom},
};

mod user_names;
use crate::user_names::USER_NAMES;

const GAME_MIN_PLAYER: usize = 2; // (inclusive)
const GAME_MAX_PLAYER: usize = 8; // (inclusive)
const GAME_CARD_START: usize = 5;
const GAME_ROUND_MAX: usize = GAME_CARD_START - 1;
const GAME_LIVE_BOT_SCHEDULE_DELAY: Duration = Duration::from_secs(10);

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

#[spacetimedb::table(accessor = user_cleaner_schedule, scheduled(user_cleaner))]
pub struct UserCleanerSchedule {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
    #[unique]
    player_id: PlayerId,
}

#[spacetimedb::table(accessor = game_lobby, public)]
pub struct GameLobby {
    #[primary_key]
    user_id: PlayerId,
    user_name: String,
    #[index(btree)]
    #[auto_inc]
    lobby_id: u32,
    ready: bool,
}

#[spacetimedb::table(accessor = game_lobby_cleaner_schedule, scheduled(game_lobby_cleaner))]
pub struct GameLobbyCleanerSchedule {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
    #[unique]
    lobby_id: u32,
}

type GameLiveId = u32; // TODO: convert to UUID
#[spacetimedb::table(accessor = game_live)]
pub struct GameLive {
    #[primary_key]
    #[auto_inc]
    id: GameLiveId,
    // TODO? Track lifecycle timestamps
    /// Track which player picked at which turn. Tail is next player to pick.
    timeline_users_id: Vec<PlayerId>,
    /// Track which card was picked at each turn. Index n is picked by timeline_users_id[n] in timeline_users_id[n+1].
    timeline_cards: Vec<Card>,
}

#[spacetimedb::table(accessor = game_live_player)]
pub struct GameLivePlayer {
    #[primary_key]
    user_id: PlayerId,
    user_name: String,
    #[index(btree)]
    game_id: GameLiveId,
    connected: bool,
    role: Role,
    cards: Vec<Card>,
    calls_bomb: bool, // TODO: Add uncalled
    calls_defuse: u8, // TODO: Add uncalled
    calls_mood: Option<Mood>,
}

#[spacetimedb::table(accessor = game_live_bot_schedule, scheduled(game_live_bot))]
pub struct GameLiveBotSchedule {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
    #[unique]
    game_id: GameLiveId,
}

#[derive(SpacetimeType)]
struct PlayerSherlock {
    name: String,
    is_moriarty: bool,
}

type GameDoneId = u32; // TODO: convert to UUID
#[spacetimedb::table(accessor = game_done, public)]
pub struct GameDone {
    #[primary_key]
    #[auto_inc]
    id: GameDoneId,
    #[index(btree)]
    finished_at: Timestamp,
    players: Box<[PlayerSherlock]>,
    timeline_users_idx: Box<[u8]>,
    timeline_cards: Box<[Card]>,
}

#[spacetimedb::table(accessor = game_done_cleaner_schedule, scheduled(game_done_cleaner))]
pub struct GameDoneCleanerSchedule {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: ScheduleAt,
    #[unique]
    game_id: GameDoneId,
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    let user = ctx.db.user().identity().find(ctx.sender());
    if let Some(user) = user {
        if let Some(mut game_live_player) = ctx.db.game_live_player().user_id().find(user.id) {
            game_live_player.connected = true;
            ctx.db.game_live_player().user_id().update(game_live_player);
        }
        ctx.db.user_cleaner_schedule().player_id().delete(user.id);
    } else {
        let random_name = USER_NAMES
            .choose(&mut ctx.rng())
            .expect("No user names available")
            .to_string();
        ctx.db.user().insert(User {
            id: 0,
            identity: ctx.sender(),
            name: random_name,
        });
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender()) {
        if let Some(mut game_lobby) = ctx.db.game_lobby().user_id().find(user.id) {
            game_lobby.ready = false;
            ctx.db.game_lobby().user_id().update(game_lobby);
        }
        if let Some(mut game_live_player) = ctx.db.game_live_player().user_id().find(user.id) {
            game_live_player.connected = false;
            ctx.db.game_live_player().user_id().update(game_live_player);
        }
        ctx.db.user_cleaner_schedule().insert(UserCleanerSchedule {
            scheduled_id: 0,
            scheduled_at: ScheduleAt::Time(ctx.timestamp + Duration::from_hours(24 * 7)),
            player_id: user.id,
        });
    }
}

#[spacetimedb::reducer]
pub fn user_update(ctx: &ReducerContext, name: String) -> Result<(), String> {
    if name.trim() != name {
        return Err("Le pseudo ne doit pas contenir d'espaces au début ou à la fin".into());
    }
    if name.is_empty() {
        return Err("Le pseudo ne doit pas être vide".into());
    }
    if name.chars().count() > 15 {
        return Err("Le pseudo ne doit pas dépasser 15 caractères".into());
    }

    let mut user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    if let Some(mut game_lobby) = ctx.db.game_lobby().user_id().find(user.id) {
        game_lobby.user_name = name.clone();
        ctx.db.game_lobby().user_id().update(game_lobby);
    }
    if let Some(mut game_live_player) = ctx.db.game_live_player().user_id().find(user.id) {
        game_live_player.user_name = name.clone();
        ctx.db.game_live_player().user_id().update(game_live_player);
    }
    user.name = name.clone();
    ctx.db.user().id().update(user);
    Ok(())
}

#[spacetimedb::view(accessor = me, public)]
fn user_me(ctx: &ViewContext) -> Option<User> {
    ctx.db.user().identity().find(ctx.sender())
}

#[spacetimedb::reducer]
fn user_cleaner(ctx: &ReducerContext, cleaner: UserCleanerSchedule) {
    ctx.db.user().id().delete(cleaner.player_id);
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
pub fn game_join(ctx: &ReducerContext, id: Option<GameLiveId>) -> Result<(), String> {
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

    let game_live = ctx.db.game_live_player().user_id().find(user.id);
    if game_live.is_some() {
        return Err("User is already in a live game".into());
    }

    if let Some(id) = id {
        let user_count = ctx.db.game_lobby().lobby_id().filter(id).count();
        if user_count > GAME_MAX_PLAYER {
            return Err("Lobby is full".into());
        }
        if user_count == 0 {
            return Err("Lobby is empty".into());
        }

        ctx.db.game_lobby().insert(GameLobby {
            user_id: user.id,
            user_name: user.name,
            lobby_id: id,
            ready: false,
        });
    } else {
        let game_lobby = ctx.db.game_lobby().insert(GameLobby {
            user_id: user.id,
            user_name: user.name,
            lobby_id: 0,
            ready: false,
        });
        ctx.db
            .game_lobby_cleaner_schedule()
            .insert(GameLobbyCleanerSchedule {
                scheduled_id: 0,
                lobby_id: game_lobby.lobby_id,
                scheduled_at: ScheduleAt::Time(ctx.timestamp + Duration::from_mins(15)),
            });
    }
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
    let lobby_id = game_lobby.lobby_id;
    ctx.db.game_lobby().user_id().update(game_lobby);
    let game_users = ctx
        .db
        .game_lobby()
        .lobby_id()
        .filter(lobby_id)
        .collect::<Box<_>>();
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

    let mut game = ctx.db.game_live().insert(GameLive {
        id: 0,
        timeline_users_id: Vec::new(),
        timeline_cards: Vec::new(),
    });
    ctx.db.game_live_bot_schedule().insert(GameLiveBotSchedule {
        scheduled_id: 0,
        game_id: game.id,
        scheduled_at: ScheduleAt::Time(ctx.timestamp + GAME_LIVE_BOT_SCHEDULE_DELAY),
    });

    let user_ids = game_users.iter().map(|g| g.user_id).collect::<Box<_>>();
    let starting_player_id = *user_ids
        .choose(&mut ctx.rng())
        .expect("user_ids is not empty");

    // TODO: Follow rules regarding roles assignations
    let mut players_roles = ROLES.iter().take(player_count).cloned().collect::<Box<_>>();
    players_roles.shuffle(&mut ctx.rng());

    let mut game_players = zip(game_users, players_roles)
        .map(|(user, player_role)| GameLivePlayer {
            user_id: user.user_id,
            user_name: user.user_name,
            game_id: game.id,
            connected: true,
            role: player_role,
            cards: Vec::new(),
            calls_bomb: false,
            calls_defuse: 0,
            calls_mood: None,
        })
        .collect::<Box<_>>();

    let mut cards = once(Card::Bomb)
        .chain(repeat_n(Card::Defuse, player_count))
        .chain(repeat(Card::Secure))
        .take(player_count * GAME_CARD_START)
        .collect::<Box<_>>();
    game_players = distribute_cards(game_players, &mut cards, &mut ctx.rng());

    game_loby_delete(ctx, lobby_id);
    game_players.into_iter().for_each(|row| {
        ctx.db.game_live_player().insert(row);
    });
    game.timeline_users_id.push(starting_player_id);
    ctx.db.game_live().id().update(game);
}

#[spacetimedb::reducer]
pub fn game_lobby_cleaner(ctx: &ReducerContext, cleaner: GameLobbyCleanerSchedule) {
    game_loby_delete(ctx, cleaner.lobby_id);
}

fn game_loby_delete(ctx: &ReducerContext, lobby_id: u32) {
    ctx.db
        .game_lobby_cleaner_schedule()
        .lobby_id()
        .delete(lobby_id);
    ctx.db.game_lobby().lobby_id().delete(lobby_id);
}

#[derive(SpacetimeType)]
pub struct MyGameLive {
    player_rows: Box<[GameLivePlayer]>,
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
    let player_game = ctx.db.game_live_player().user_id().find(user.id)?;
    let game_players = ctx
        .db
        .game_live_player()
        .game_id()
        .filter(player_game.game_id)
        .collect::<Box<_>>();
    let game = ctx
        .db
        .game_live()
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
    let game = ctx
        .db
        .game_live()
        .id()
        .find(game_picker.game_id)
        .expect("Game not found");
    if game.timeline_users_id[game.timeline_users_id.len() - 1] != user.id {
        panic!("Not your turn"); // TODO: error
    }

    let game_players = ctx
        .db
        .game_live_player()
        .game_id()
        .filter(game_picker.game_id)
        .collect::<Box<_>>();

    game_pick_card_inner(ctx, game, game_players, pick_player_id, card_idx);
}

#[spacetimedb::reducer]
pub fn game_live_bot(ctx: &ReducerContext, schedule: GameLiveBotSchedule) {
    let game = ctx
        .db
        .game_live()
        .id()
        .find(schedule.game_id)
        .expect("Game not found");
    let game_players = ctx
        .db
        .game_live_player()
        .game_id()
        .filter(game.id)
        .collect::<Box<_>>();
    let current_player_id = game
        .timeline_users_id
        .last()
        .expect("Timeline should not be empty");
    let (pick_player_id, card_idx) = loop {
        if let Some(pick_player) = game_players.choose(&mut ctx.rng()) {
            if pick_player.user_id == *current_player_id {
                continue;
            }
            let card_idx = ctx.rng().gen_range(0..pick_player.cards.len());
            break (pick_player.user_id, card_idx);
        }
    };

    game_pick_card_inner(ctx, game, game_players, pick_player_id, card_idx);
}

fn game_pick_card_inner(
    ctx: &ReducerContext,
    mut game: GameLive,
    mut game_players: Box<[GameLivePlayer]>,
    pick_player_id: u32,
    card_idx: usize,
) {
    ctx.db.game_live_bot_schedule().game_id().delete(game.id);
    let player_count = game_players.len();

    let game_picked = game_players
        .iter_mut()
        .find(|gp| gp.user_id == pick_player_id)
        .expect("Players are not in the same game");
    let card = game_picked.cards.remove(card_idx); // TODO: graceful error?
    game.timeline_users_id.push(game_picked.user_id);
    game.timeline_cards.push(card);

    if card == Card::Bomb {
        process_game_finished(ctx, &game, &game_players);
        return;
    }

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
        game_picked.calls_defuse = game_picked.calls_defuse.saturating_sub(1);
    }

    if game.timeline_cards.len() >= player_count * GAME_ROUND_MAX {
        process_game_finished(ctx, &game, &game_players);
        return;
    }

    let game_players = if game.timeline_cards.len().is_multiple_of(player_count) {
        let mut cards = game_players
            .iter()
            .flat_map(|gp| gp.cards.clone())
            .collect::<Box<_>>();
        distribute_cards(game_players, &mut cards, &mut ctx.rng())
    } else {
        game_players
    };

    for gp in game_players {
        ctx.db.game_live_player().user_id().update(gp);
    }

    let game_id = game.id;
    ctx.db.game_live().id().update(game);
    ctx.db.game_live_bot_schedule().insert(GameLiveBotSchedule {
        scheduled_id: 0,
        game_id,
        scheduled_at: ScheduleAt::Time(ctx.timestamp + GAME_LIVE_BOT_SCHEDULE_DELAY),
    });
}

fn distribute_cards(
    mut game_players: Box<[GameLivePlayer]>,
    cards: &mut Box<[Card]>,
    rng: &mut impl Rng,
) -> Box<[GameLivePlayer]> {
    cards.shuffle(rng);
    let cards_per_player = cards.len() / game_players.len();
    game_players
        .iter_mut()
        .zip(cards.chunks_exact(cards_per_player))
        .for_each(|(gp, new_cards)| {
            gp.cards = new_cards.to_vec();
            gp.calls_bomb = false;
            gp.calls_defuse = 0;
            gp.calls_mood = None;
        });
    game_players
}

#[spacetimedb::reducer]
fn game_call_mood(ctx: &ReducerContext, mood: Option<Mood>) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.calls_mood = mood;
    ctx.db.game_live_player().user_id().update(player_game);
}

#[spacetimedb::reducer]
fn game_call_defuse(ctx: &ReducerContext, defuse: u8) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.calls_defuse = defuse;
    ctx.db.game_live_player().user_id().update(player_game);
}

#[spacetimedb::reducer]
fn game_call_bomb(ctx: &ReducerContext, bomb: bool) {
    let (_user, mut player_game) = relove_user_game_live(ctx);
    player_game.calls_bomb = bomb;
    ctx.db.game_live_player().user_id().update(player_game);
}

fn process_game_finished(ctx: &ReducerContext, game: &GameLive, game_players: &[GameLivePlayer]) {
    ctx.db.game_live_player().game_id().delete(game.id);
    ctx.db.game_live().id().delete(game.id);

    let mut user_ids = game_players.iter().map(|gp| gp.user_id).collect::<Box<_>>();
    user_ids.sort();
    let inserted_game = ctx.db.game_done().insert(GameDone {
        id: 0,
        finished_at: ctx.timestamp,
        players: game_players
            .iter()
            .map(|gp| PlayerSherlock {
                name: gp.user_name.to_string(),
                is_moriarty: matches!(gp.role, Role::Moriarty0 | Role::Moriarty1 | Role::Moriarty2),
            })
            .collect::<Box<_>>(),
        timeline_users_idx: game
            .timeline_users_id
            .iter()
            .map(|user_id| user_ids.binary_search(user_id).unwrap() as u8)
            .collect::<Box<_>>(),
        timeline_cards: game.timeline_cards.clone().into_boxed_slice(),
    });
    ctx.db
        .game_done_cleaner_schedule()
        .insert(GameDoneCleanerSchedule {
            scheduled_id: 0,
            scheduled_at: ScheduleAt::Time(ctx.timestamp + Duration::from_hours(24)),
            game_id: inserted_game.id,
        });
}

fn relove_user_game_live(ctx: &ReducerContext) -> (User, GameLivePlayer) {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender())
        .expect("User not found");
    let player_game = ctx
        .db
        .game_live_player()
        .user_id()
        .find(user.id)
        .expect("User not in live game");

    (user, player_game)
}

#[spacetimedb::reducer]
pub fn game_done_cleaner(ctx: &ReducerContext, schedule: GameDoneCleanerSchedule) {
    ctx.db.game_done().id().delete(schedule.game_id);
    ctx.db
        .game_done_cleaner_schedule()
        .scheduled_id()
        .delete(schedule.scheduled_id);
}
