# PendingItemSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **String** | Pending item ID. | 
**account** | **String** | Account username. | 
**source** | [**models::PendingItemSource**](PendingItemSource.md) | Source of the pending item. | 
**source_id** | Option<**String**> | ID reference for the source (e.g., achievement code, order id). | [optional]
**description** | **String** | Description for display. | 
**gold** | Option<**i32**> | Gold amount. | [optional][default to 0]
**items** | Option<[**Vec<models::SimpleItemSchema>**](SimpleItemSchema.md)> | List of items to be claimed. | [optional]
**created_at** | **String** | When the pending item was created. | 
**claimed_at** | Option<**String**> | When the pending item was claimed. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


