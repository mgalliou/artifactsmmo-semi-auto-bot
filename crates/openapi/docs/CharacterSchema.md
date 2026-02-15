# CharacterSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** | Name of the character. | 
**account** | **String** | Account name. | 
**skin** | [**models::CharacterSkin**](CharacterSkin.md) | Character skin code. | 
**level** | **i32** | Combat level. | 
**xp** | **i32** | The current xp level of the combat level. | 
**max_xp** | **i32** | XP required to level up the character. | 
**gold** | **i32** | The numbers of gold on this character. | 
**speed** | **i32** | *Not available, on the roadmap. Character movement speed. | 
**mining_level** | **i32** | Mining level. | 
**mining_xp** | **i32** | The current xp level of the Mining skill. | 
**mining_max_xp** | **i32** | Mining XP required to level up the skill. | 
**woodcutting_level** | **i32** | Woodcutting level. | 
**woodcutting_xp** | **i32** | The current xp level of the Woodcutting skill. | 
**woodcutting_max_xp** | **i32** | Woodcutting XP required to level up the skill. | 
**fishing_level** | **i32** | Fishing level. | 
**fishing_xp** | **i32** | The current xp level of the Fishing skill. | 
**fishing_max_xp** | **i32** | Fishing XP required to level up the skill. | 
**weaponcrafting_level** | **i32** | Weaponcrafting level. | 
**weaponcrafting_xp** | **i32** | The current xp level of the Weaponcrafting skill. | 
**weaponcrafting_max_xp** | **i32** | Weaponcrafting XP required to level up the skill. | 
**gearcrafting_level** | **i32** | Gearcrafting level. | 
**gearcrafting_xp** | **i32** | The current xp level of the Gearcrafting skill. | 
**gearcrafting_max_xp** | **i32** | Gearcrafting XP required to level up the skill. | 
**jewelrycrafting_level** | **i32** | Jewelrycrafting level. | 
**jewelrycrafting_xp** | **i32** | The current xp level of the Jewelrycrafting skill. | 
**jewelrycrafting_max_xp** | **i32** | Jewelrycrafting XP required to level up the skill. | 
**cooking_level** | **i32** | The current xp level of the Cooking skill. | 
**cooking_xp** | **i32** | Cooking XP. | 
**cooking_max_xp** | **i32** | Cooking XP required to level up the skill. | 
**alchemy_level** | **i32** | Alchemy level. | 
**alchemy_xp** | **i32** | Alchemy XP. | 
**alchemy_max_xp** | **i32** | Alchemy XP required to level up the skill. | 
**hp** | **i32** | Character actual HP. | 
**max_hp** | **i32** | Character max HP. | 
**haste** | **i32** | *Increase speed attack (reduce fight cooldown) | 
**critical_strike** | **i32** | % Critical strike. Critical strikes adds 50% extra damage to an attack (1.5x). | 
**wisdom** | **i32** | Wisdom increases the amount of XP gained from fights and skills (1% extra per 10 wisdom). | 
**prospecting** | **i32** | Prospecting increases the chances of getting drops from fights and skills (1% extra per 10 PP). | 
**initiative** | **i32** | Initiative determines turn order in combat. Higher initiative goes first. | 
**threat** | **i32** | Threat level affects monster targeting in multi-character combat. | 
**attack_fire** | **i32** | Fire attack. | 
**attack_earth** | **i32** | Earth attack. | 
**attack_water** | **i32** | Water attack. | 
**attack_air** | **i32** | Air attack. | 
**dmg** | **i32** | % Damage. Damage increases your attack in all elements. | 
**dmg_fire** | **i32** | % Fire damage. Damage increases your fire attack. | 
**dmg_earth** | **i32** | % Earth damage. Damage increases your earth attack. | 
**dmg_water** | **i32** | % Water damage. Damage increases your water attack. | 
**dmg_air** | **i32** | % Air damage. Damage increases your air attack. | 
**res_fire** | **i32** | % Fire resistance. Reduces fire attack. | 
**res_earth** | **i32** | % Earth resistance. Reduces earth attack. | 
**res_water** | **i32** | % Water resistance. Reduces water attack. | 
**res_air** | **i32** | % Air resistance. Reduces air attack. | 
**effects** | Option<[**Vec<models::StorageEffectSchema>**](StorageEffectSchema.md)> | List of active effects on the character. | [optional]
**x** | **i32** | Character x coordinate. | 
**y** | **i32** | Character y coordinate. | 
**layer** | [**models::MapLayer**](MapLayer.md) | Character current layer. | 
**map_id** | **i32** | Character current map ID. | 
**cooldown** | **i32** | Cooldown in seconds. | 
**cooldown_expiration** | Option<**String**> | Datetime Cooldown expiration. | [optional]
**weapon_slot** | **String** | Weapon slot. | 
**rune_slot** | **String** | Rune slot. | 
**shield_slot** | **String** | Shield slot. | 
**helmet_slot** | **String** | Helmet slot. | 
**body_armor_slot** | **String** | Body armor slot. | 
**leg_armor_slot** | **String** | Leg armor slot. | 
**boots_slot** | **String** | Boots slot. | 
**ring1_slot** | **String** | Ring 1 slot. | 
**ring2_slot** | **String** | Ring 2 slot. | 
**amulet_slot** | **String** | Amulet slot. | 
**artifact1_slot** | **String** | Artifact 1 slot. | 
**artifact2_slot** | **String** | Artifact 2 slot. | 
**artifact3_slot** | **String** | Artifact 3 slot. | 
**utility1_slot** | **String** | Utility 1 slot. | 
**utility1_slot_quantity** | **u32** | Utility 1 quantity. | 
**utility2_slot** | **String** | Utility 2 slot. | 
**utility2_slot_quantity** | **u32** | Utility 2 quantity. | 
**bag_slot** | **String** | Bag slot. | 
**task** | **String** | Task in progress. | 
**task_type** | **String** | Task type. | 
**task_progress** | **i32** | Task progression. | 
**task_total** | **i32** | Task total objective. | 
**inventory_max_items** | **i32** | Inventory max items. | 
**inventory** | Option<[**Vec<models::InventorySlot>**](InventorySlot.md)> | List of inventory slots. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


