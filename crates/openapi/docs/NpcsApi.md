# \NpcsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_npcs_items_npcs_items_get**](NpcsApi.md#get_all_npcs_items_npcs_items_get) | **GET** /npcs/items | Get All Npcs Items
[**get_all_npcs_npcs_details_get**](NpcsApi.md#get_all_npcs_npcs_details_get) | **GET** /npcs/details | Get All Npcs
[**get_npc_items_npcs_items_code_get**](NpcsApi.md#get_npc_items_npcs_items_code_get) | **GET** /npcs/items/{code} | Get Npc Items
[**get_npc_npcs_details_code_get**](NpcsApi.md#get_npc_npcs_details_code_get) | **GET** /npcs/details/{code} | Get Npc



## get_all_npcs_items_npcs_items_get

> models::DataPageNpcItem get_all_npcs_items_npcs_items_get(code, npc, currency, page, size)
Get All Npcs Items

Retrieve the list of all NPC items.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | Option<**String**> | Item code. |  |
**npc** | Option<**String**> | NPC code. |  |
**currency** | Option<**String**> | Currency code. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageNpcItem**](DataPage_NPCItem_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_all_npcs_npcs_details_get

> models::DataPageNpcSchema get_all_npcs_npcs_details_get(name, r#type, page, size)
Get All Npcs

Fetch NPCs details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | Option<**String**> | NPC name. |  |
**r#type** | Option<[**models::NpcType**](.md)> | Type of NPCs. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageNpcSchema**](DataPage_NPCSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_npc_items_npcs_items_code_get

> models::DataPageNpcItem get_npc_items_npcs_items_code_get(code, page, size)
Get Npc Items

Retrieve the items list of a NPC. If the NPC has items to buy, sell or trade, they will be displayed.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the NPC. | [required] |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageNpcItem**](DataPage_NPCItem_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_npc_npcs_details_code_get

> models::NpcResponseSchema get_npc_npcs_details_code_get(code)
Get Npc

Retrieve the details of a NPC.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the NPC. | [required] |

### Return type

[**models::NpcResponseSchema**](NPCResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

