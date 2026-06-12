function {{ func.name }}({{ func.args_decl }}){{ func.return_type_decl }}
{
    {%- if func.has_return %}
    {%- if func.throws_type_spec != "" %}
    $result = UniFFIRuntime::rustCallWithError('{{ func.ffi_name }}', {{ func.throws_type_spec }}{% if func.args_call != "" %}, {{ func.args_call }}{% endif %});
    {%- else %}
    $result = UniFFIRuntime::rustCall('{{ func.ffi_name }}'{% if func.args_call != "" %}, {{ func.args_call }}{% endif %});
    {%- endif %}
    return {{ func.lift_expr }};
    {%- else %}
    {%- if func.throws_type_spec != "" %}
    UniFFIRuntime::rustCallWithError('{{ func.ffi_name }}', {{ func.throws_type_spec }}{% if func.args_call != "" %}, {{ func.args_call }}{% endif %});
    {%- else %}
    UniFFIRuntime::rustCall('{{ func.ffi_name }}'{% if func.args_call != "" %}, {{ func.args_call }}{% endif %});
    {%- endif %}
    {%- endif %}
}
