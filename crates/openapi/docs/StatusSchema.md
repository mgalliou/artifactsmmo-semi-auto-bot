# StatusSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**version** | **String** | Game version. | 
**server_time** | **String** | Server time. | 
**max_level** | **i32** | Maximum level. | 
**max_skill_level** | **i32** | Maximum skill level. | 
**characters_online** | **i32** | Characters online. | 
**season** | Option<[**models::SeasonSchema**](SeasonSchema.md)> | Current season details. | [optional]
**rate_limits** | [**Vec<models::RateLimitSchema>**](RateLimitSchema.md) | Rate limits. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


