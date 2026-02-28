# AccountAchievementSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** | Name of the achievement. | 
**code** | **String** | Code of the achievement. | 
**description** | **String** | Description of the achievement. | 
**points** | **i32** | Points of the achievement. Used for the leaderboard. | 
**objectives** | [**Vec<models::AccountAchievementObjectiveSchema>**](AccountAchievementObjectiveSchema.md) | List of objectives with progress. | 
**rewards** | [**models::AchievementRewardsSchema**](AchievementRewardsSchema.md) | Rewards. | 
**completed_at** | Option<**String**> | Completion timestamp. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


