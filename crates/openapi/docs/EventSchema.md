# EventSchema

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **String** | Name of the event. | 
**code** | **String** | Code of the event. | 
**content** | [**models::EventContentSchema**](EventContentSchema.md) | Content of the event. | 
**maps** | [**Vec<models::EventMapSchema>**](EventMapSchema.md) | Map list of the event. | 
**duration** | **i32** | Duration in minutes. | 
**rate** | **i32** | Rate spawn of the event. (1/rate every minute) | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


