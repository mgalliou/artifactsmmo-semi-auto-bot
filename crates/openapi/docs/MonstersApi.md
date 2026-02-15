# \MonstersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_monsters_monsters_get**](MonstersApi.md#get_all_monsters_monsters_get) | **GET** /monsters | Get All Monsters
[**get_monster_monsters_code_get**](MonstersApi.md#get_monster_monsters_code_get) | **GET** /monsters/{code} | Get Monster



## get_all_monsters_monsters_get

> models::DataPageMonsterSchema get_all_monsters_monsters_get(name, min_level, max_level, drop, page, size)
Get All Monsters

Fetch monsters details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | Option<**String**> | Name of the monster. |  |
**min_level** | Option<**u32**> | Minimum level. |  |
**max_level** | Option<**u32**> | Maximum level. |  |
**drop** | Option<**String**> | Item code of the drop. |  |
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageMonsterSchema**](DataPage_MonsterSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_monster_monsters_code_get

> models::MonsterResponseSchema get_monster_monsters_code_get(code)
Get Monster

Retrieve the details of a monster.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the monster. | [required] |

### Return type

[**models::MonsterResponseSchema**](MonsterResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

