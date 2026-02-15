# \ItemsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_items_items_get**](ItemsApi.md#get_all_items_items_get) | **GET** /items | Get All Items
[**get_item_items_code_get**](ItemsApi.md#get_item_items_code_get) | **GET** /items/{code} | Get Item



## get_all_items_items_get

> models::DataPageItemSchema get_all_items_items_get(name, min_level, max_level, r#type, craft_skill, craft_material, page, size)
Get All Items

Fetch items details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | Option<**String**> | Name of the item. |  |
**min_level** | Option<**u32**> | Minimum level. |  |
**max_level** | Option<**u32**> | Maximum level. |  |
**r#type** | Option<[**models::ItemType**](.md)> | Type of items. |  |
**craft_skill** | Option<[**models::CraftSkill**](.md)> | Skill to craft items. |  |
**craft_material** | Option<**String**> | Item code of items used as material for crafting. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageItemSchema**](DataPage_ItemSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_item_items_code_get

> models::ItemResponseSchema get_item_items_code_get(code)
Get Item

Retrieve the details of a item.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the item. | [required] |

### Return type

[**models::ItemResponseSchema**](ItemResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

