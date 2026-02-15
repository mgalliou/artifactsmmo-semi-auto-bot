# LogSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**character** | **String** | Character name. | 
**account** | **String** | Account character. | 
**r#type** | [**models::LogType**](LogType.md) | Type of action. | 
**description** | **String** | Description of action. | 
**content** | Option<[**serde_json::Value**](.md)> |  | 
**cooldown** | **i32** | Cooldown in seconds. | 
**cooldown_expiration** | Option<**String**> | Datetime of cooldown expiration. | [optional]
**created_at** | **String** | Datetime of creation. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


