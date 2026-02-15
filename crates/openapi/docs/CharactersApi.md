# \CharactersApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_character_characters_create_post**](CharactersApi.md#create_character_characters_create_post) | **POST** /characters/create | Create Character
[**delete_character_characters_delete_post**](CharactersApi.md#delete_character_characters_delete_post) | **POST** /characters/delete | Delete Character
[**get_active_characters_characters_active_get**](CharactersApi.md#get_active_characters_characters_active_get) | **GET** /characters/active | Get Active Characters
[**get_character_characters_name_get**](CharactersApi.md#get_character_characters_name_get) | **GET** /characters/{name} | Get Character



## create_character_characters_create_post

> models::CharacterResponseSchema create_character_characters_create_post(add_character_schema)
Create Character

Create new character on your account. You can create up to 5 characters.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**add_character_schema** | [**AddCharacterSchema**](AddCharacterSchema.md) |  | [required] |

### Return type

[**models::CharacterResponseSchema**](CharacterResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_character_characters_delete_post

> models::CharacterResponseSchema delete_character_characters_delete_post(delete_character_schema)
Delete Character

Delete character on your account.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**delete_character_schema** | [**DeleteCharacterSchema**](DeleteCharacterSchema.md) |  | [required] |

### Return type

[**models::CharacterResponseSchema**](CharacterResponseSchema.md)

### Authorization

[JWTBearer](../README.md#JWTBearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_active_characters_characters_active_get

> models::DataPageActiveCharacterSchema get_active_characters_characters_active_get(page, size)
Get Active Characters

Fetch active characters details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageActiveCharacterSchema**](DataPage_ActiveCharacterSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_character_characters_name_get

> models::CharacterResponseSchema get_character_characters_name_get(name)
Get Character

Retrieve the details of a character.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | **String** | The name of the character. | [required] |

### Return type

[**models::CharacterResponseSchema**](CharacterResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

