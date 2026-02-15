# ItemSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** | Item name. | 
**code** | **String** | Item code. This is the item's unique identifier (ID). | 
**level** | **u32** | Item level. | 
**r#type** | **String** | Item type. | 
**subtype** | **String** | Item subtype. | 
**description** | **String** | Item description. | 
**conditions** | Option<[**Vec<models::ConditionSchema>**](ConditionSchema.md)> | Item conditions. If applicable. Conditions for using or equipping the item. | [optional]
**effects** | Option<[**Vec<models::SimpleEffectSchema>**](SimpleEffectSchema.md)> | List of object effects. For equipment, it will include item stats. | [optional]
**craft** | Option<[**models::CraftSchema**](CraftSchema.md)> | Craft information. If applicable. | [optional]
**tradeable** | **bool** | Item tradeable status. A non-tradeable item cannot be exchanged or sold. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


