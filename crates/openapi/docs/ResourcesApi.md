# \ResourcesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_resources_resources_get**](ResourcesApi.md#get_all_resources_resources_get) | **GET** /resources | Get All Resources
[**get_resource_resources_code_get**](ResourcesApi.md#get_resource_resources_code_get) | **GET** /resources/{code} | Get Resource



## get_all_resources_resources_get

> models::DataPageResourceSchema get_all_resources_resources_get(min_level, max_level, skill, drop, page, size)
Get All Resources

Fetch resources details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**min_level** | Option<**u32**> | Minimum level. |  |
**max_level** | Option<**u32**> | Maximum level. |  |
**skill** | Option<[**models::GatheringSkill**](.md)> | Skill of resources. |  |
**drop** | Option<**String**> | Item code of the drop. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageResourceSchema**](DataPage_ResourceSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_resource_resources_code_get

> models::ResourceResponseSchema get_resource_resources_code_get(code)
Get Resource

Retrieve the details of a resource.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the resource. | [required] |

### Return type

[**models::ResourceResponseSchema**](ResourceResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

