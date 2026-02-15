# \EventsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_active_events_events_active_get**](EventsApi.md#get_all_active_events_events_active_get) | **GET** /events/active | Get All Active Events
[**get_all_events_events_get**](EventsApi.md#get_all_events_events_get) | **GET** /events | Get All Events
[**spawn_event_events_spawn_post**](EventsApi.md#spawn_event_events_spawn_post) | **POST** /events/spawn | Spawn Event



## get_all_active_events_events_active_get

> models::DataPageActiveEventSchema get_all_active_events_events_active_get(page, size)
Get All Active Events

Fetch active events details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageActiveEventSchema**](DataPage_ActiveEventSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_all_events_events_get

> models::DataPageEventSchema get_all_events_events_get(r#type, page, size)
Get All Events

Fetch events details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**r#type** | Option<[**models::MapContentType**](.md)> | Type of events. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageEventSchema**](DataPage_EventSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## spawn_event_events_spawn_post

> models::ActiveEventResponseSchema spawn_event_events_spawn_post(spawn_event_request)
Spawn Event

Spawn a specific event by code consuming 1 event token.  Rules:   - Maximum active events defined by utils.config.max_active_events().   - Event must not already be active.   - Member or founder account required.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**spawn_event_request** | [**SpawnEventRequest**](SpawnEventRequest.md) |  | [required] |

### Return type

[**models::ActiveEventResponseSchema**](ActiveEventResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

