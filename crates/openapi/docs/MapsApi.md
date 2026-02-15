# \MapsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_maps_maps_get**](MapsApi.md#get_all_maps_maps_get) | **GET** /maps | Get All Maps
[**get_layer_maps_maps_layer_get**](MapsApi.md#get_layer_maps_maps_layer_get) | **GET** /maps/{layer} | Get Layer Maps
[**get_map_by_id_maps_id_map_id_get**](MapsApi.md#get_map_by_id_maps_id_map_id_get) | **GET** /maps/id/{map_id} | Get Map By Id
[**get_map_by_position_maps_layer_xy_get**](MapsApi.md#get_map_by_position_maps_layer_xy_get) | **GET** /maps/{layer}/{x}/{y} | Get Map By Position



## get_all_maps_maps_get

> models::DataPageMapSchema get_all_maps_maps_get(layer, content_type, content_code, hide_blocked_maps, page, size)
Get All Maps

Fetch maps details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**layer** | Option<[**models::MapLayer**](.md)> | Filter maps by layer. |  |
**content_type** | Option<[**models::MapContentType**](.md)> | Type of maps. |  |
**content_code** | Option<**String**> | Content code on the map. |  |
**hide_blocked_maps** | Option<**bool**> | When true, excludes maps with access_type 'blocked' from the results. |  |[default to false]
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageMapSchema**](DataPage_MapSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_layer_maps_maps_layer_get

> models::DataPageMapSchema get_layer_maps_maps_layer_get(layer, content_type, content_code, hide_blocked_maps, page, size)
Get Layer Maps

Fetch maps details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**layer** | **String** | The layer of the map (interior, overworld, underground). | [required] |
**content_type** | Option<**String**> | Type of maps. |  |
**content_code** | Option<**String**> | Content code on the map. |  |
**hide_blocked_maps** | Option<**bool**> | When true, excludes maps with access_type 'blocked' from the results. |  |[default to false]
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageMapSchema**](DataPage_MapSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_map_by_id_maps_id_map_id_get

> models::MapResponseSchema get_map_by_id_maps_id_map_id_get(map_id)
Get Map By Id

Retrieve the details of a map by its unique ID.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**map_id** | **i32** | The unique ID of the map. | [required] |

### Return type

[**models::MapResponseSchema**](MapResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_map_by_position_maps_layer_xy_get

> models::MapResponseSchema get_map_by_position_maps_layer_xy_get(layer, x, y)
Get Map By Position

Retrieve the details of a map by layer and coordinates.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**layer** | **String** | The layer of the map (interior, overworld, underground). | [required] |
**x** | **i32** | The position x of the map. | [required] |
**y** | **i32** | The position y of the map. | [required] |

### Return type

[**models::MapResponseSchema**](MapResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

