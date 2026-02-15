# \EffectsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_all_effects_effects_get**](EffectsApi.md#get_all_effects_effects_get) | **GET** /effects | Get All Effects
[**get_effect_effects_code_get**](EffectsApi.md#get_effect_effects_code_get) | **GET** /effects/{code} | Get Effect



## get_all_effects_effects_get

> models::DataPageEffectSchema get_all_effects_effects_get(page, size)
Get All Effects

List of all effects. Effects are used by equipment, tools, runes, consumables and monsters. An effect is an action that produces an effect on the game.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**u32**> | Page number |  |[default to 1]
**size** | Option<**u32**> | Page size |  |[default to 50]

### Return type

[**models::DataPageEffectSchema**](DataPage_EffectSchema_.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_effect_effects_code_get

> models::EffectResponseSchema get_effect_effects_code_get(code)
Get Effect

Retrieve the details of an effect.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**code** | **String** | The code of the effect. | [required] |

### Return type

[**models::EffectResponseSchema**](EffectResponseSchema.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

