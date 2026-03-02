use bevy::prelude::*;

/// Top-level application state machine.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Main-menu splash screen; shown on startup.
    #[default]
    MainMenu,
    /// Load game slot picker shown from MainMenu.
    LoadGameMenu,
    /// Practice scenario picker shown when starting Practice mode.
    ScenarioSelect,
    /// Campaign slot picker and naming screen shown when starting Campaign mode.
    CampaignSelect,
    /// Active simulation / gameplay.
    Playing,
    /// Simulation frozen; in-game pause overlay is visible.
    Paused,
    /// Ore shop open; simulation frozen, consumable upgrades available.
    OreShop,
    /// Player has exhausted all lives; game-over overlay shown.
    GameOver,
}

/// Tracks which state to return to when the ore shop is closed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShopReturnState {
    #[default]
    Playing,
    Paused,
}

/// Which scenario the player has chosen to play.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedScenario {
    /// Asteroid-only seeded clustered field with varied starts.
    #[default]
    Field,
    /// Strong central planetoid with varied orbital debris rings.
    Orbit,
    /// Large-biased outer-annulus bodies with gentle inward crossing flow.
    Comets,
    /// Dense small-body outer shower with inward rain trajectories.
    Shower,
}

/// Active top-level gameplay mode selected from the main menu.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedGameMode {
    /// Scenario-driven free-play mode with immediate sandbox access.
    #[default]
    Practice,
    /// Mission-progression mode with campaign flow.
    Campaign,
}

/// Root node of the main-menu UI; entire tree is despawned on `OnExit(MainMenu)`.
#[derive(Component)]
pub struct MainMenuRoot;

/// Tags the "Start Game" button.
#[derive(Component)]
pub struct MenuPracticeButton;

/// Tags the "Campaign" button.
#[derive(Component)]
pub struct MenuCampaignButton;

/// Tags the "Load Game" button.
#[derive(Component)]
pub struct MenuLoadButton;

/// Tags the "Quit" button.
#[derive(Component)]
pub struct MenuQuitButton;

/// Root node of the load-game screen; despawned on `OnExit(LoadGameMenu)`.
#[derive(Component)]
pub struct LoadGameRoot;

/// Tags the "Load Slot 1" button.
#[derive(Component)]
pub struct LoadSlot1Button;

/// Tags the "Load Slot 2" button.
#[derive(Component)]
pub struct LoadSlot2Button;

/// Tags the "Load Slot 3" button.
#[derive(Component)]
pub struct LoadSlot3Button;

/// Tags the "Back" button on the load-game screen.
#[derive(Component)]
pub struct LoadGameBackButton;

/// Root node of the scenario-select screen; despawned on `OnExit(ScenarioSelect)`.
#[derive(Component)]
pub struct ScenarioSelectRoot;

/// Root node of the campaign-select screen; despawned on `OnExit(CampaignSelect)`.
#[derive(Component)]
pub struct CampaignSelectRoot;

/// Tags the "Field" scenario button.
#[derive(Component)]
pub struct ScenarioFieldButton;

/// Tags the "Orbit" scenario button.
#[derive(Component)]
pub struct ScenarioOrbitButton;

/// Tags the "Comets" scenario button.
#[derive(Component)]
pub struct ScenarioCometButton;

/// Tags the "Shower" scenario button.
#[derive(Component)]
pub struct ScenarioShowerButton;

/// Tags the "Back" button on the scenario-select screen.
#[derive(Component)]
pub struct ScenarioBackButton;

/// Tags the campaign slot 1 button.
#[derive(Component)]
pub struct CampaignSlot1Button;

/// Tags the campaign slot 2 button.
#[derive(Component)]
pub struct CampaignSlot2Button;

/// Tags the campaign slot 3 button.
#[derive(Component)]
pub struct CampaignSlot3Button;

/// Tags the campaign start/resume button.
#[derive(Component)]
pub struct CampaignStartButton;

/// Tags the campaign save-name button.
#[derive(Component)]
pub struct CampaignSaveNameButton;

/// Tags the campaign back button.
#[derive(Component)]
pub struct CampaignBackButton;

/// Dynamic text showing currently selected campaign slot.
#[derive(Component)]
pub struct CampaignSelectedSlotText;

/// Dynamic text showing campaign slot name edit buffer.
#[derive(Component)]
pub struct CampaignNameValueText;

/// Name-editor resource for campaign slot naming UX.
#[derive(Resource, Debug, Clone)]
pub struct CampaignNameEditor {
    pub selected_slot: u8,
    pub buffer: String,
}

impl Default for CampaignNameEditor {
    fn default() -> Self {
        Self {
            selected_slot: 1,
            buffer: "Campaign Slot 1".to_string(),
        }
    }
}

/// Root node of the pause-menu overlay; entire tree is despawned on `OnExit(Paused)`.
#[derive(Component)]
pub struct PauseMenuRoot;

/// Tags the "Resume" button in the pause menu.
#[derive(Component)]
pub struct PauseResumeButton;

/// Tags the "Debug Overlays" toggle button in the pause menu.
#[derive(Component)]
pub struct PauseDebugButton;

/// Tags the "Main Menu" button in the pause menu.
#[derive(Component)]
pub struct PauseMainMenuButton;

/// Tags the "SAVE SLOT 1" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot1Button;

/// Tags the "SAVE SLOT 2" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot2Button;

/// Tags the "SAVE SLOT 3" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot3Button;

/// Root node of the ore shop overlay; despawned when the shop is closed.
#[derive(Component)]
pub struct OreShopRoot;

/// Tags the "BUY HEALTH" button inside the ore shop.
#[derive(Component)]
pub struct OreShopHealButton;

/// Tags the "BUY MISSILE" button inside the ore shop.
#[derive(Component)]
pub struct OreShopMissileButton;

/// Tags the "CLOSE" button inside the ore shop.
#[derive(Component)]
pub struct OreShopCloseButton;

/// Tags the ore count text in the ore shop.
#[derive(Component)]
pub struct OreShopOreText;

/// Tags the health row status text.
#[derive(Component)]
pub struct OreShopHealText;

/// Tags the missile row status text.
#[derive(Component)]
pub struct OreShopMissileText;

/// Tags the "BUY UPGRADE" button inside the unified ore shop.
#[derive(Component)]
pub struct OreShopUpgradeButton;

/// Tags the missile upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopMissileUpgradeButton;

/// Tags the magnet upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopMagnetUpgradeButton;

/// Tags the tractor beam upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopTractorUpgradeButton;

/// Tags the ion cannon upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopIonUpgradeButton;

/// Root node of the game-over overlay; despawned on `OnExit(GameOver)`.
#[derive(Component)]
pub struct GameOverRoot;

/// Tags the "Play Again" button in the game-over overlay.
#[derive(Component)]
pub struct GameOverPlayAgainButton;
